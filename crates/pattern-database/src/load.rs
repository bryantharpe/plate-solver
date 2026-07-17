#[derive(Debug)]
pub struct LoadError;

pub fn load_from_path(_path: &std::path::Path) -> Result<(), LoadError> {
    Err(LoadError)
}
