use super::parse::INesParseError;

macro_rules! mappers {
    ($($name:ident,)*) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum INesMapper {
            $($name,)*
        }

        impl INesMapper {
            pub fn from_id(id: u8) -> Result<Self, INesParseError> {
                #[allow(non_upper_case_globals)]
                mod ids {
                    $(pub const $name: u8 = super::INesMapper::$name as u8;)*
                }
                match id {
                    $(ids::$name => Ok(Self::$name),)*
                    _ => Err(INesParseError::UnsupportedMapper(id))
                }
            }
        }
    };
}

mappers! {
    NRom,
}
