#[cfg(windows)]
fn main() -> std::io::Result<()>{
    let res = winres::WindowsResource::new();
    res.compile()?;
    Ok(())
}

#[cfg(not(windows))]
fn main() {
}