// Functions to convert from coordinates to tiles and vice versa
// From: https://wiki.openstreetmap.org/wiki/Slippy_map_tilenames

pub fn deg2num(lat_deg: f64, lon_deg: f64, zoom: u32) -> (u32, u32) {
    let lat_rad = lat_deg.to_radians();
    let n = 2.0_f64.powi(zoom as i32);
    let xtile = ((lon_deg + 180.0) / 360.0 * n).floor() as u32;
    let ytile = ((1.0 - lat_rad.tan().asinh() / std::f64::consts::PI) / 2.0 * n).floor() as u32;
    return (xtile, ytile);
}


pub fn num2deg(xtile: u32, ytile: u32, zoom: u32) -> (f64, f64) {
    let n = 2.0_f64.powi(zoom as i32);
    let lon_deg = (xtile as f64) / n * 360.0 - 180.0;
    let lat_rad = (std::f64::consts::PI * (1. - 2. * ytile as f64 / n)).sinh().atan();
    let lat_deg = lat_rad.to_degrees();
    return (lat_deg, lon_deg);
}
