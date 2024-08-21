pub trait Validator {
    fn get_validation_options(
    ) -> Result<mongodb::options::CreateCollectionOptions, Box<dyn std::error::Error>>;
}
