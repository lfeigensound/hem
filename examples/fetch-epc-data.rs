use home_energy_model::fetch_scottish_epc_data;
use std::fs;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("Fetching Scottish EPC data from statistics.gov.scot...");
    
    let data = fetch_scottish_epc_data().await?;
    
    println!("✓ Fetched {} records", data.record_count);
    println!("✓ Last updated: {}", data.last_updated);
    
    let json = serde_json::to_string_pretty(&data)?;
    fs::write("epc-data.json", json)?;
    
    println!("✓ Data saved to epc-data.json");
    
    Ok(())
}
