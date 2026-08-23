#![allow(
    clippy::too_many_arguments,
    clippy::doc_overindented_list_items,
    dead_code
)]

pub mod compare_floats;
pub mod core;
pub mod corpus;
pub mod data_fetchers;
pub mod errors;

pub mod hem_core;

pub mod input;
mod input_dependency_resolvers;
pub mod output;
pub mod output_writer;
pub mod read_weather_file;
pub mod statistics;

pub use data_fetchers::{fetch_scottish_epc_data, EPCDataset, EPCRecord};

use crate::core::units::{convert_profile_to_daily, WATTS_PER_KILOWATT};
use crate::corpus::{
    Corpus, NumberOrDivisionByZero, OutputOptions, ResultsAnnual, ResultsPerTimestep,
};
use crate::errors::{HemCoreError, HemError, NotImplementedError};
use crate::external_conditions::ExternalConditions;
use crate::input::{ExternalConditionsInput, HotWaterSourceDetails, Input};
use crate::output::{Output, OutputEmitters, OutputStatic, OUTPUT_ZONE_DATA_FIELD_HEADINGS};
use crate::output_writer::OutputWriter;
use crate::read_weather_file::{
    cibse_weather_data_to_external_conditions, epw_weather_data_to_external_conditions,
    ExternalConditions as ExternalConditionsFromFile, ReadWeatherFileResult,
};
use crate::simulation_time::SimulationTime;
use anyhow::{anyhow, bail};
use approx::relative_eq;
use convert_case::{Case, Casing};
use csv::WriterBuilder;
use erased_serde::Serialize as ErasedSerialize;
use hem_core::external_conditions;
use hem_core::simulation_time;
use indexmap::IndexMap;
use itertools::Itertools;
use jsonschema::Validator;
use serde::{Deserialize, Serialize, Serializer};
use serde_json::Value;
use smartstring::alias::String;
use std::borrow::Cow;
use std::fmt::{Debug, Display, Formatter};
use std::io::Read;
use std::ops::AddAssign;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::{Arc, LazyLock};
use thiserror::Error;
use tracing::{debug, instrument};

pub const HEM_VERSION: &str = "1.0.0a7";
pub const HEM_VERSION_DATE: &str = "2026-02-27";