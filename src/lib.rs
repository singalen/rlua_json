use std::fmt::{Display, Formatter};

use mlua::{FromLua, IntoLua};
use serde_json::{json, Value as JsonValue, Value};
use serde::{Deserialize, Serialize};

/// Because you cannot impl an external trait for an external struct.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct JsonWrapperValue(JsonValue);

impl Display for JsonWrapperValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl JsonWrapperValue {
    pub fn new(value: JsonValue) -> Self {
        JsonWrapperValue(value)
    }

    pub fn from(value: &JsonValue) -> Self {
        JsonWrapperValue(value.clone())
    }

    pub fn into_map(self) -> serde_json::Map<String, serde_json::Value> {
        match self.0 {
            Value::Object(o) => o,
            _ => panic!("Cannot convert non-object to map"),
        }
    }
}

impl From<JsonValue> for JsonWrapperValue {
    fn from(val: JsonValue) -> Self {
        JsonWrapperValue::new(val)
    }
}

impl Into<JsonValue> for JsonWrapperValue {
    fn into(self) -> JsonValue { self.0 }
}

impl<'lua> IntoLua for JsonWrapperValue {
    fn into_lua(self, lua: &mlua::Lua) -> mlua::Result<mlua::Value> {
        let result = match self.into() {
            JsonValue::Null => mlua::Value::Nil,
            JsonValue::String(s) => s.as_str().into_lua(lua)?,
            JsonValue::Number(n) => {

                if let Some(ni) = n.as_i64() {
                    return ni.into_lua(lua);
                }

                (
                    n.as_f64().ok_or_else(|| mlua::Error::ToLuaConversionError {
                        from: "JsonValue::Number".to_string(),
                        to: "Value::Number",
                        message: None,
                    })? as f64
                ).into_lua(lua)?
            },
            JsonValue::Bool(b) => b.into_lua(lua)?,
            JsonValue::Object(o) => {
                let iter = o.into_iter()
                    .map(|(k, v)| (k, JsonWrapperValue::new(v.clone())));
                mlua::Value::Table(
                    lua.create_table_from(iter)?
                )
            },
            JsonValue::Array(a) => {
                let iter = a.into_iter()
                    .map(|it| JsonWrapperValue::new(it));
                mlua::Value::Table(
                    lua.create_table_from(iter.enumerate())?
                )
            },
        };

        Ok(result)
    }
}

impl FromLua for JsonWrapperValue {
    fn from_lua(lua_value: mlua::Value, lua: &mlua::Lua) -> mlua::Result<Self> {
        let result = match lua_value {
            mlua::Value::Nil => JsonValue::Null,
            mlua::Value::Boolean(b) => JsonValue::Bool(b),
            mlua::Value::LightUserData(_) => return Err(
                mlua::Error::FromLuaConversionError {
                    from: "LightUserData", to: "JsonValue".to_string(), message: Some("Impossible to convert".to_string()) }),
            mlua::Value::Integer(i) => JsonValue::from(i),
            mlua::Value::Number(n) => JsonValue::from(n),
            mlua::Value::String(s) => JsonValue::from(s.to_str()?.as_ref()),
            mlua::Value::Table(t) => {
                let mut o = json!({});
                for pair in t.pairs::<mlua::String, mlua::Value>() {
                    let (key, value) = pair?;
                    let key = key.to_str()?;
                    let value = JsonWrapperValue::from_lua(value, lua)?.0;
                    o
                        .as_object_mut()
                        .unwrap()
                        .insert(key.to_string(), value);
                }
                o
            }
            mlua::Value::Function(_) => return Err(
                mlua::Error::FromLuaConversionError {
                    from: "Function", to: "JsonValue".to_string(), message: Some("Impossible to convert".to_string()) }),
            mlua::Value::Thread(_) => return Err(
                mlua::Error::FromLuaConversionError {
                    from: "Thread", to: "JsonValue".to_string(), message: Some("Impossible to convert".to_string()) }),
            mlua::Value::UserData(_) => return Err(
                mlua::Error::FromLuaConversionError {
                    from: "UserData", to: "JsonValue".to_string(), message: Some("Impossible to convert".to_string()) }),
            mlua::Value::Error(_) => return Err(
                mlua::Error::FromLuaConversionError {
                    from: "Error", to: "JsonValue".to_string(), message: Some("Impossible to convert".to_string()) }),
            mlua::Value::Other(_) => return Err(
                mlua::Error::FromLuaConversionError {
                    from: "Other", to: "JsonValue".to_string(), message: Some("Impossible to convert".to_string()) }),
        };

        return Ok( JsonWrapperValue(result) )
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use mlua::{Lua, IntoLua, FromLua, Value};
    use crate::JsonWrapperValue;

    #[test]
    fn object_string_values() {

        let source_table = json!({"foo": "bar"});
        let source_table = JsonWrapperValue::new(source_table);

        let lua_ctx = Lua::new();

        let rlua_table = source_table.into_lua(&lua_ctx)
            .expect("table");
        match &rlua_table {
            Value::Table(t) => {
                assert_eq!(t.get::<String>("foo".to_string())
                               .expect("foo"),
                           "bar");
                t.set("from_lua", "string value")
                    .expect("table.set() failed");
            },
            _ => panic!("table.to_lua() didn't return a Table"),
        }

        let resulting_table = JsonWrapperValue::from_lua(rlua_table, &lua_ctx)
            .map(|it| it.0)
            .expect("JsonWrapperValue::from_lua failed");

        assert!(resulting_table.is_object());
        assert_eq!(resulting_table["from_lua"].as_str(), Some("string value"));
    }

    // TODO: A lot more tests, including tests for error reporting on invalid data.
}

