use crate::repositry::application::{AppVersionPo, ApplicaionPo};

use super::{AppVersioned, Application};

impl<'a> From<&'a Application> for ApplicaionPo<'a> {
    fn from(value: &'a Application) -> Self {
        ApplicaionPo {
            id: value.id,
            name: (&value.name).into(),
            git_url: (&value.git).into(),
        }
    }
}

impl TryFrom<(ApplicaionPo<'static>, Vec<AppVersionPo<'static>>)> for Application {
    type Error = anyhow::Error;

    fn try_from(value: (ApplicaionPo<'static>, Vec<AppVersionPo>)) -> Result<Self, Self::Error> {
        let (app, versions) = value;

        let app = Application {
            id: app.id,
            name: app.name.into_owned(),
            git: app.git_url.into_owned(),
            versions: versions
                .into_iter()
                .map(|v| {
                    let AppVersionPo { hash, app_id: _ } = v;
                    AppVersioned { hash: hash.into_owned() }
                })
                .collect(),
        };
        Ok(app)
    }
}
