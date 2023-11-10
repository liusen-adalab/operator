#[cfg(not(feature = "diesel"))]
#[macro_export]
macro_rules! diesel_new_type {
    ($($tt:tt)*) => {};
}
#[cfg(not(feature = "diesel"))]
#[macro_export]
macro_rules! diesel_enum {
    ($($tt:tt)*) => {};
}

#[cfg(feature = "diesel")]
#[macro_export]
macro_rules! diesel_new_type {
    ($type:ty, $pg_type:ty) => {
        const _: () = {
            use diesel::{
                backend::Backend,
                deserialize::{self, FromSql},
                serialize::{self, Output, ToSql},
            };
            type Pg = diesel::sqlite::Sqlite;

            impl ToSql<$pg_type, Pg> for $type {
                fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
                    ToSql::<$pg_type, Pg>::to_sql(&self.0, out)
                }
            }

            impl FromSql<$pg_type, Pg> for $type {
                fn from_sql(bytes: <Pg as Backend>::RawValue<'_>) -> deserialize::Result<Self> {
                    let res = FromSql::<$pg_type, Pg>::from_sql(bytes)?;
                    Ok(Self(res))
                }
            }
        };
    };

    ($type:ty, $pg_type:ty, try_from: $map_ty:ty) => {
        const _: () = {
            use diesel::{
                backend::Backend,
                deserialize::{self, FromSql},
                serialize::{self, Output, ToSql},
            };

            type Pg = diesel::pg::Pg;

            impl ToSql<$pg_type, Pg> for $type {
                fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
                    let map_to = <$map_ty>::try_from(self)?;
                    ToSql::<$pg_type, Pg>::to_sql(&map_to, &mut out.reborrow())
                }
            }

            impl FromSql<$pg_type, Pg> for $type {
                fn from_sql(bytes: <Pg as Backend>::RawValue<'_>) -> deserialize::Result<Self> {
                    let res: $map_ty = FromSql::<$pg_type, Pg>::from_sql(bytes)?;
                    Ok(Self::try_from(res)?)
                }
            }
        };
    };
}

#[cfg(feature = "diesel")]
#[macro_export]
macro_rules! diesel_enum {
    ($enum:ty, max = $max:expr) => {
        impl<'a> From<&'a $enum> for i16 {
            fn from(value: &'a $enum) -> Self {
                *value as i16
            }
        }

        impl TryFrom<i16> for $enum {
            type Error = anyhow::Error;

            fn try_from(value: i16) -> std::result::Result<Self, Self::Error> {
                ::anyhow::ensure!(
                    value >= 0,
                    "db corrupted. value should greater than 0: {}",
                    value
                );
                ::anyhow::ensure!(value <= $max, "db enum out of bound. Max = {} Value = {}", $max, value);

                unsafe { Ok(std::mem::transmute(value)) }
            }
        }

        $crate::diesel_new_type!($enum, ::diesel::sql_types::SmallInt, try_from: i16);
    };
}

#[cfg(test)]
mod test {
    use diesel::{deserialize::FromSqlRow, expression::AsExpression};

    #[derive(Debug, Clone, Copy, FromSqlRow, AsExpression)]
    #[diesel(sql_type = ::diesel::sql_types::SmallInt)]
    #[repr(i16)]
    pub enum TaskStatusPo {
        Accepted,
        Running,
        Finished,
        Failed,
        Canceled,
        Reported,
    }

    diesel_enum!(TaskStatusPo, max = TaskStatusPo::Reported as i16);
}
