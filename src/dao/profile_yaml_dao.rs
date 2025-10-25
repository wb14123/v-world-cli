use crate::dao::profile_dao::ProfileDao;
use crate::model::profile::Profile;
use std::error::Error;
use std::path::Path;
use tokio::fs::{create_dir, try_exists, File};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub struct ProfileYamlDao {
    db_path: String,
}

pub(crate) async fn new(db_path: String) -> Result<ProfileYamlDao, Box<dyn Error>> {
    let path = Path::new(&db_path);
    if !try_exists(&path).await? {
        create_dir(&path).await?;
    }
    Ok(ProfileYamlDao { db_path })
}

impl ProfileDao for ProfileYamlDao {

    async fn create(&self, profile: &Profile) -> Result<bool, Box<dyn Error>> {
        let output_path = Path::new(&self.db_path).join(&profile.id).with_extension("yaml");
        if try_exists(&output_path).await? {
            return Ok(false);
        }
        let mut file = File::create(output_path).await?;
        let yaml_content = serde_yaml::to_string(&profile)?;
        file.write_all(yaml_content.as_bytes()).await?;
        Ok(true)
    }

    async fn get(&self, id: &String) -> Result<Option<Profile>, Box<dyn Error>> {
        let yaml_file = Path::new(&self.db_path).join(&id).with_extension("yaml");
        if !try_exists(&yaml_file).await? {
            return Ok(None)
        }
        let mut f = File::open(yaml_file).await?;
        let mut contents = String::new();
        f.read_to_string(&mut contents).await?;
        Ok(Some(serde_yaml::from_str(&contents)?))
    }

}

