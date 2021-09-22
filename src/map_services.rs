use super::coord_utils::*;
use bytes::Buf;
use std::fs::File;

async fn download_to_file(url: &str, filename: &str) -> Result<(), Box<dyn std::error::Error>> {
    match File::open(filename) {
        Ok(_) => {
            println!("File exists");
            Ok(())
        }
        Err(_) => {
            let client = reqwest::Client::new();
            let resp = client
                .get(url)
                .header("Referer", "https://www.google.com/maps")
                .send()
                .await?
                .bytes()
                .await?;
            let mut file = File::create(filename)?;
            ::std::io::copy(&mut resp.reader(), &mut file)?; //TODO: Delete file if failed?
            Ok(())
        }
    }
}

pub async fn get_opentopography_tile(
    x: u32,
    y: u32,
    z: u32,
) -> Result<String, Box<dyn std::error::Error>> {
    let (north, west) = num2deg(x, y, z);
    let (south, east) = num2deg(x + 1, y + 1, z);
    let url = format!("https://portal.opentopography.org/API/globaldem?demtype=SRTMGL1&west={}&east={}&south={}&north={}&outputFormat=GTiff", west, east, south, north);
    println!("{}", url);
    let filename = format!("assets/images/topo_{}_{}_{}.tiff", x, y, z);
    download_to_file(&url, &filename).await?;
    Ok(filename)
}

// TODO: figure out why are the lerc files clamped :(
pub async fn _get_arcgis_topo_tile(
    x: u32,
    y: u32,
    z: u32,
) -> Result<String, Box<dyn std::error::Error>> {
    let url = format!("https://services.arcgisonline.com/arcgis/rest/services/WorldElevation3D/Terrain3D/ImageServer/tile/{}/{}/{}", z, y, x);
    let filename = format!("images/topo_{}_{}_{}.lerc", x, y, z);
    download_to_file(&url, &filename).await?;
    Ok(filename)
}

pub async fn get_arcgis_image_tile(
    x: u32,
    y: u32,
    z: u32,
) -> Result<String, Box<dyn std::error::Error>> {
    let url = format!("https://server.arcgisonline.com/ArcGIS/rest/services/World_Imagery/MapServer/tile/{}/{}/{}", z, y, x);
    println!("{}", url);
    let filename = format!("assets/images/imagery_{}_{}_{}.jpeg", x, y, z);
    download_to_file(&url, &filename).await?;
    println!("Done!");
    Ok(filename)
}
