use crate::repositry::host::HostPo;

use super::Host;

impl<'a> From<&'a Host> for HostPo<'a> {
    fn from(value: &'a Host) -> Self {
        to_po(value)
    }
}

impl TryFrom<HostPo<'static>> for Host {
    type Error = anyhow::Error;

    fn try_from(value: HostPo<'static>) -> Result<Self, Self::Error> {
        Ok(from_po(value))
    }
}

pub fn to_po(host: &Host) -> HostPo {
    HostPo {
        id: host.id,
        name: (&host.name).into(),
        ip: host.ip.to_string().into(),
    }
}

pub fn from_po(po: HostPo) -> Host {
    Host {
        id: po.id,
        name: po.name.into_owned(),
        ip: po.ip.parse().unwrap(),
        state: super::HostState::Disconnected,
    }
}
