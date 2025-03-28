mod data_sink;
mod data_source;
mod run;

pub mod transfer;

pub use self::{
    data_sink::{DataSink, DataSinkCountBytes},
    data_source::{DataSource, DataSourceRandom, DataSourceRead, DataSourceZeroes},
    run::RunOptions,
};

pub const DEFAULT_DEVICE: &str = "qdmac1000";
