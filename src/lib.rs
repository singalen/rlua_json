use rlua;
use rlua::{Context, FromLua, ToLua};
use json::JsonValue;

/// Because you cannot impl an external trait for an external struct.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct JsonWrapperValue {
    value: JsonValue,
}

impl JsonWrapperValue {
    pub fn new(value: JsonValue) -> Self {
        JsonWrapperValue { value }
    }
}

impl<'lua> ToLua<'lua> for JsonWrapperValue {
    fn to_lua(self, lua: Context<'lua>) -> rlua::Result<rlua::Value<'lua>> {
        let result = match self.value {
            JsonValue::Null => rlua::Value::Nil,
            JsonValue::Short(s) => s.as_str().to_lua(lua)?,
            JsonValue::String(s) => s.as_str().to_lua(lua)?,
            JsonValue::Number(n) => (
                (n.as_fixed_point_i64(2).ok_or_else(|| rlua::Error::ToLuaConversionError {
                    from: "JsonValue::Number",
                    to: "Value::Number",
                    message: None
                })? as f64) * 0.01
            ).to_lua(lua)?,
            JsonValue::Boolean(b) => b.to_lua(lua)?,
            JsonValue::Object(o) => {
                let iter = o.iter()
                    .map(|(k, v)| (k, JsonWrapperValue::new(v.clone())));
                rlua::Value::Table(
                    lua.create_table_from(iter)?
                )
            },
            JsonValue::Array(a) => {
                let iter = a.into_iter()
                    .map(|it| JsonWrapperValue::new(it));
                rlua::Value::Table(
                    lua.create_table_from(iter.enumerate())?
                )
            },
        };

        Ok(result)
    }
}

impl<'lua> FromLua<'lua> for JsonWrapperValue {
    fn from_lua(lua_value: rlua::Value<'lua>, lua: Context<'lua>) -> rlua::Result<Self> {
        let result = match lua_value {
            rlua::Value::Nil => JsonValue::Null,
            rlua::Value::Boolean(b) => JsonValue::Boolean(b),
            rlua::Value::LightUserData(_) => return Err(
                rlua::Error::FromLuaConversionError {
                    from: "LightUserData", to: "JsonValue", message: Some("Impossible to convert".to_string()) }),
            rlua::Value::Integer(i) => JsonValue::from(i),
            rlua::Value::Number(n) => JsonValue::from(n),
            rlua::Value::String(s) => JsonValue::from(s.to_str()?),
            rlua::Value::Table(t) => {
                let mut o = JsonValue::new_object();
                for pair in t.pairs::<rlua::String, rlua::Value>() {
                    let (key, value) = pair?;
                    let key = key.to_str()?;
                    let value = JsonWrapperValue::from_lua(value, lua)?.value;
                    o.insert(key, value)
                        .map_err(|e| rlua::Error::ToLuaConversionError{
                            from: "JsonObject",
                            to: "insert",
                            message: Some(format!("{}", e))
                        })?;
                }
                o
            }
            rlua::Value::Function(_) => return Err(
                rlua::Error::FromLuaConversionError {
                    from: "Function", to: "JsonValue", message: Some("Impossible to convert".to_string()) }),
            rlua::Value::Thread(_) => return Err(
                rlua::Error::FromLuaConversionError {
                    from: "Thread", to: "JsonValue", message: Some("Impossible to convert".to_string()) }),
            rlua::Value::UserData(_) => return Err(
                rlua::Error::FromLuaConversionError {
                    from: "UserData", to: "JsonValue", message: Some("Impossible to convert".to_string()) }),
            rlua::Value::Error(_) => return Err(
                rlua::Error::FromLuaConversionError {
                    from: "Error", to: "JsonValue", message: Some("Impossible to convert".to_string()) }),
        };

        return Ok( JsonWrapperValue { value: result } )
    }
}

#[cfg(test)]
mod tests {
    use json::JsonValue;
    use rlua::{Lua, ToLua, FromLua, Value};
    use crate::JsonWrapperValue;

    #[test]
    fn object_string_values() {

        let mut source_table = JsonValue::new_object();
        source_table.insert("foo", "bar")
            .expect("insert");

        let source_table = JsonWrapperValue::new(source_table);

        let lua = Lua::new();
        let resulting_table = lua.context(|lua_ctx| {

            let rlua_table = source_table.to_lua(lua_ctx)
                .expect("table");
            match &rlua_table {
                Value::Table(t) => {
                    assert_eq!(t.get::<_, String>("foo")
                                   .expect("foo"),
                               "bar");
                    t.set("from_lua", "string value")
                        .expect("table.set() failed");
                },
                _ => panic!("table.to_lua() didn't return a Table"),
            }

            JsonWrapperValue::from_lua(rlua_table, lua_ctx)
                .map(|it| it.value)
        }).expect("JsonWrapperValue::from_lua failed");

        assert!(resulting_table.is_object());
        assert!(resulting_table.has_key("from_lua"));
        assert_eq!(resulting_table["from_lua"], "string value");
    }

    // TODO: A lot more tests, including tests for error reporting on invalid data.
}

