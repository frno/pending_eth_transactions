use std::str::FromStr;
use anyhow::Result;
use web3::types::{Address, H160};

#[allow(dead_code)]
pub fn convert_to_address(other: &str) -> Result<Address> {
    let address = &other[2..];
    let address: core::result::Result<Address, <H160 as FromStr>::Err > = address.parse();
    match address{
        Ok(val) => {
            Ok(val)
        }
        Err(err) => {
            Err(anyhow::Error::new(err))
        }
    }
}