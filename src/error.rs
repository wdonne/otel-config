use config::ConfigError;
use log::{ParseLevelError, SetLoggerError};
use opentelemetry_otlp::ExporterBuildError;
use opentelemetry_sdk::error::OTelSdkError;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("config error: {0}")]
    Config(#[from] ConfigError),
    #[error("exporter error: {0}")]
    Exporter(#[from] ExporterBuildError),
    #[error("otel sdk error: {0}")]
    OtelSfk(#[from] OTelSdkError),
    #[error("parse log level error: {0}")]
    ParseLogLevel(#[from] ParseLevelError),
    #[error("set logger error: {0}")]
    SetLogger(#[from] SetLoggerError),
}
