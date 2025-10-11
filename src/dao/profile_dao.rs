use std::error::Error;
use crate::model::profile::Profile;

pub trait ProfileDao {
    async fn create(&self, profile: &Profile) -> Result<bool, Box<dyn Error>>;
    async fn get(&self, id: &String) -> Result<Option<Profile>, Box<dyn Error>>;
}
