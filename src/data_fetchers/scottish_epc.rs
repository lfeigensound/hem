use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// Represents a single EPC record from the Scottish EPC Registry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EPCRecord {
    pub uprn: String,
    pub property_type: String,
    pub postcode: String,
    pub address: String,
    pub energy_rating: i32,
    pub environmental_rating: i32,
    pub sap_score: f64,
    pub main_fuel_type: String,
    pub floor_area: f64,
    pub assessment_date: String,
}

/// Complete EPC dataset with metadata
#[derive(Debug, Serialize, Deserialize)]
pub struct EPCDataset {
    pub records: Vec<EPCRecord>,
    pub last_updated: String,
    pub record_count: usize,
}

/// Fetches EPC data from statistics.gov.scot via SPARQL API
///
/// This function queries the Scottish domestic energy performance certificates dataset
/// and returns structured data for use in HEM calculations.
pub async fn fetch_scottish_epc_data() -> Result<EPCDataset> {
    let sparql_query = r#"
PREFIX qb: <http://purl.org/linked-data/cube#>
PREFIX sdmx: <http://purl.org/linked-data/sdmx/dimension#>
PREFIX sdmxDim: <http://purl.org/linked-data/sdmx/dimension#>
PREFIX ps: <http://purl.org/linked-data/sdmx/property#>
PREFIX skos: <http://www.w3.org/2004/02/skos/core#>

SELECT ?propertyType ?energyRating ?sAPScore ?mainFuelType ?floorArea ?assessmentDate
WHERE {
    ?obs qb:dataSet <http://statistics.gov.scot/data/domestic-energy-performance-certificates> ;
         sdmxDim:refArea ?area ;
         <http://purl.org/linked-data/sdmx/measure#obsValue> ?value .
    
    OPTIONAL { ?obs <http://statistics.gov.scot/def/dimension/propertyType> ?propertyType . }
    OPTIONAL { ?obs <http://statistics.gov.scot/def/dimension/energyRating> ?energyRating . }
    OPTIONAL { ?obs <http://statistics.gov.scot/def/dimension/sAPScore> ?sAPScore . }
    OPTIONAL { ?obs <http://statistics.gov.scot/def/dimension/mainFuelType> ?mainFuelType . }
    OPTIONAL { ?obs <http://statistics.gov.scot/def/dimension/floorArea> ?floorArea . }
    OPTIONAL { ?obs sdmxDim:timePeriod ?assessmentDate . }
}
LIMIT 100
"#;

    let client = reqwest::Client::new();
    let response = client
        .get("https://statistics.gov.scot/sparql")
        .query(&[("query", sparql_query), ("output", "json")])
        .send()
        .await
        .context("Failed to connect to statistics.gov.scot SPARQL endpoint")?;

    if !response.status().is_success() {
        return Err(anyhow::anyhow!(
            "SPARQL API error: {}",
            response.status()
        ));
    }

    let data: serde_json::Value = response
        .json()
        .await
        .context("Failed to parse SPARQL response")?;

    let records = parse_epc_response(&data)?;
    let record_count = records.len();

    Ok(EPCDataset {
        records,
        last_updated: chrono::Utc::now().to_rfc3339(),
        record_count,
    })
}

/// Parses the SPARQL JSON response into structured EPC records
fn parse_epc_response(data: &serde_json::Value) -> Result<Vec<EPCRecord>> {
    let bindings = data
        .get("results")
        .and_then(|r| r.get("bindings"))
        .and_then(|b| b.as_array())
        .context("Invalid SPARQL response: missing results.bindings array")?;

    let records = bindings
        .iter()
        .filter_map(|binding| {
            let get_str = |key: &str| -> Option<String> {
                binding
                    .get(key)?
                    .get("value")?
                    .as_str()
                    .map(|s| s.to_string())
            };

            let get_f64 = |key: &str| -> Option<f64> {
                binding
                    .get(key)?
                    .get("value")?
                    .as_str()?
                    .parse()
                    .ok()
            };

            let get_i32 = |key: &str| -> Option<i32> {
                binding
                    .get(key)?
                    .get("value")?
                    .as_str()?
                    .parse()
                    .ok()
            };

            Some(EPCRecord {
                uprn: get_str("uprn").unwrap_or_default(),
                property_type: get_str("propertyType").unwrap_or_default(),
                postcode: get_str("postcode").unwrap_or_default(),
                address: get_str("address").unwrap_or_default(),
                energy_rating: get_i32("energyRating").unwrap_or(-1),
                environmental_rating: get_i32("environmentalRating").unwrap_or(-1),
                sap_score: get_f64("sAPScore").unwrap_or(0.0),
                main_fuel_type: get_str("mainFuelType").unwrap_or_default(),
                floor_area: get_f64("floorArea").unwrap_or(0.0),
                assessment_date: get_str("assessmentDate").unwrap_or_default(),
            })
        })
        .collect();

    Ok(records)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_epc_response_empty() {
        let json = serde_json::json!({
            "results": {
                "bindings": []
            }
        });
        let records = parse_epc_response(&json).unwrap();
        assert_eq!(records.len(), 0);
    }

    #[test]
    fn test_parse_epc_response_with_data() {
        let json = serde_json::json!({
            "results": {
                "bindings": [
                    {
                        "propertyType": { "value": "Detached house" },
                        "energyRating": { "value": "4" },
                        "sAPScore": { "value": "75.5" },
                        "mainFuelType": { "value": "Gas" },
                        "floorArea": { "value": "120.5" },
                        "assessmentDate": { "value": "2023-01-15" }
                    }
                ]
            }
        });
        let records = parse_epc_response(&json).unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].property_type, "Detached house");
        assert_eq!(records[0].energy_rating, 4);
        assert_eq!(records[0].sap_score, 75.5);
    }
}
