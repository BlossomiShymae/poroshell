pub mod error;
pub mod reader;
pub mod types;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reader() -> Result<(), error::Error> {
        let _ = reader::load(
            "https://raw.githubusercontent.com/BlossomiShymae/poroschema/refs/heads/main/schemas/lcu.json",
        )?;
        let _ = reader::load(
            "https://raw.githubusercontent.com/BlossomiShymae/poroschema/refs/heads/main/schemas/lolclient.json",
        )?;
        let _ = reader::load(
            "https://raw.githubusercontent.com/BlossomiShymae/poroschema/refs/heads/main/schemas/riotapi.json",
        )?;

        Ok(())
    }
}
