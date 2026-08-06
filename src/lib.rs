//! Utility to set up OpenTelemetry providers from a configuration file.
use crate::error::Error;
use config::Config;
use log::{Level, LevelFilter};
use multi_log::MultiLogger;
use opentelemetry::{KeyValue, StringValue, Value};
use opentelemetry_appender_log::OpenTelemetryLogBridge;
use opentelemetry_otlp::{
    Compression, LogExporter, MetricExporter, SpanExporter, WithExportConfig, WithTonicConfig,
};
use opentelemetry_sdk::{
    Resource, logs::SdkLoggerProvider, metrics::SdkMeterProvider, trace::SdkTracerProvider,
};
use simple_logger::SimpleLogger;
use std::str::FromStr;

pub mod error;

/// Combines the three OpenTelemetry providers.
pub struct Providers {
    pub logger_provider: SdkLoggerProvider,
    pub meter_provider: SdkMeterProvider,
    pub tracer_provider: SdkTracerProvider,
}

impl Providers {
    /// Creates the three providers with the configuration it finds below `path`. The subfield
    /// `endpoint` should contain the OpenTelemetry collector URL. The subfield `name` identifies
    /// the OpenTelemetry resource. Attributes can be added to the resource with the `attributes`
    /// subfield, which is an object. In the following configuration example, the `otel` field
    /// would be the value of the `path` argument.
    /// ```
    /// otel:
    ///   endpoint: "http://my-collector:4317"
    ///   name: my-app
    ///   attributes:
    ///     service.namespace: apps
    /// ```
    pub fn form_config(config: &Config, path: &str) -> Result<Self, Error> {
        let endpoint = config.get_string(&(path.to_string() + ".endpoint"))?;
        let resource = resource(config, path)?;

        Ok(Self {
            logger_provider: SdkLoggerProvider::builder()
                .with_batch_exporter(
                    LogExporter::builder()
                        .with_tonic()
                        .with_endpoint(&endpoint)
                        .with_compression(Compression::Gzip)
                        .build()?,
                )
                .with_resource(resource.clone())
                .build(),
            meter_provider: SdkMeterProvider::builder()
                .with_periodic_exporter(
                    MetricExporter::builder()
                        .with_tonic()
                        .with_endpoint(&endpoint)
                        .with_compression(Compression::Gzip)
                        .build()?,
                )
                .with_resource(resource.clone())
                .build(),
            tracer_provider: SdkTracerProvider::builder()
                .with_batch_exporter(
                    SpanExporter::builder()
                        .with_tonic()
                        .with_endpoint(&endpoint)
                        .with_compression(Compression::Gzip)
                        .build()?,
                )
                .with_resource(resource.clone())
                .build(),
        })
    }

    /// Shut down all providers.
    pub fn shutdown(&self) -> Result<(), Error> {
        self.logger_provider.shutdown()?;
        self.meter_provider.shutdown()?;
        self.tracer_provider.shutdown()?;
        Ok(())
    }
}

fn attributes(config: &Config, path: &str) -> Vec<KeyValue> {
    config
        .get_table(&(path.to_string() + ".attributes"))
        .ok()
        .map(|m| {
            m.iter()
                .filter_map(|(k, v)| v.clone().into_string().ok().map(|s| (k, s)))
                .map(|(k, v)| KeyValue::new(k.to_string(), Value::String(StringValue::from(v))))
                .collect()
        })
        .unwrap_or_default()
}

/// If the configuration contains the field `log-level`, which can have the values defined in the
/// [log](https://docs.rs/log/latest/log/) crate, then the logger is set up to send logs to both the
/// [SimpleLogger](https://docs.rs/simple_logger/latest/simple_logger/struct.SimpleLogger.html) and
/// the OpenTelemetry log provider.
pub fn logging(config: &Config, logger_provider: Option<&SdkLoggerProvider>) -> Result<(), Error> {
    let simple = match config
        .get_string("log-level")
        .ok()
        .map(|l| LevelFilter::from_str(&l))
    {
        Some(l) => SimpleLogger::new().with_level(l?),
        None => SimpleLogger::new().env(),
    };

    match logger_provider.map(|p| Box::new(OpenTelemetryLogBridge::new(p))) {
        Some(l) => MultiLogger::init(vec![Box::new(simple), l], Level::Debug)?,
        None => simple.init()?,
    };
    Ok(())
}

fn resource(config: &Config, path: &str) -> Result<Resource, Error> {
    Ok(Resource::builder()
        .with_service_name(config.get_string(&(path.to_string() + ".name"))?)
        .with_attributes(attributes(config, path))
        .build())
}
