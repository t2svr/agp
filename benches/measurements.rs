// Copyright 2024 Junshuang Hu
use std::time::{Duration, Instant};

use criterion::{measurement::{Measurement, ValueFormatter},Throughput};

pub struct RxxOPS;
impl Measurement for RxxOPS {
    type Intermediate = Instant;
    type Value = Duration;

    fn start(&self) -> Self::Intermediate {
        Instant::now()
    }
    fn end(&self, i: Self::Intermediate) -> Self::Value {
        i.elapsed()
    }
    fn add(&self, v1: &Self::Value, v2: &Self::Value) -> Self::Value {
        *v1 + *v2
    }
    fn zero(&self) -> Self::Value {
        Duration::from_millis(0)
    }
    fn to_f64(&self, val: &Self::Value) -> f64 {
        val.as_millis() as f64
    }
    fn formatter(&self) -> &dyn ValueFormatter {
        &RxxOPSFormatter
    }
}

pub struct RxxOPSFormatter;
impl ValueFormatter for RxxOPSFormatter {
    fn scale_values(&self, _: f64, _: &mut [f64]) -> &'static str {
       "μs"
    }

    fn scale_throughputs(
        &self,
        _: f64,
        throughput: &Throughput,
        values: &mut [f64],
    ) -> &'static str {
        match *throughput {
            Throughput::Bytes(_) =>{
                "Unsupported"
            },
            Throughput::BytesDecimal(_) => {
                "Unsupported"
            }
            Throughput::Elements(elems) => {
                for v in values {
                    *v =  elems as f64 * 4_000.0 / *v;
                }
                "rules/sec"
            },
        }
    }

    fn scale_for_machines(&self, _: &mut [f64]) -> &'static str {
        "μs"
    }
}
