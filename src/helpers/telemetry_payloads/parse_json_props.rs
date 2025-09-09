use std::convert::TryFrom;
use std::str::FromStr;

pub fn get_float_number_array_prop(prop_a: &serde_json::Value) -> Result<Vec<Option<f64>>, String> {
    // let prop_a = match prop_o { Some(a) => a, None => return Ok(vec![None;array_length]) };
    if let Some(vec) = prop_a.as_array() {
        let mut arr = Vec::<Option<f64>>::new();
        arr.reserve(vec.len());
        for prop_a in vec {
            if let Some(v) = prop_a.as_f64() {
                arr.push(Some(v));
            } else if let Some(v) = prop_a.as_i64() {
                arr.push(Some(f64::from(i32::try_from(v).unwrap())));
            } else if let Some(v) = prop_a.as_str() {
                arr.push(match f64::from_str(v) {
                    Ok(v) => Some(v),
                    Err(_err) => None,
                });
            } else {
                arr.push(None);
            }
        }
        return Ok(arr);
    } else if let Some(vec_str) = prop_a.as_str() {
        let vec: serde_json::Value =
            serde_json::from_str(vec_str).map_err(|e| format!("EROR22 {}", e))?;
        if let Some(vec) = vec.as_array() {
            let mut arr = Vec::<Option<f64>>::new();
            arr.reserve(vec.len());
            for prop_a in vec {
                if let Some(v) = prop_a.as_f64() {
                    arr.push(Some(v));
                } else if let Some(v) = prop_a.as_i64() {
                    arr.push(Some(f64::from(i32::try_from(v).unwrap())));
                } else {
                    arr.push(None);
                }
            }
            return Ok(arr);
        }
    }
    return Err("Could not find valid attribute value".to_owned());
}

pub fn get_float_array_optional(prop_a: &serde_json::Value) -> Option<Vec<Option<f64>>> {
    // O esperado é que a propriedade seja um array
    if let Some(vec) = prop_a.as_array() {
        let mut arr: Vec<Option<f64>> = Vec::with_capacity(vec.len());
        for prop_a in vec {
            let v = get_float_number_optional(prop_a);
            arr.push(v);
        }
        return Some(arr);
    }

    // Se não for array e for um valor numérico, a gente transforma em array com 1 valor
    if let Some(v) = get_float_number_optional(prop_a) {
        return Some(vec![Some(v)]);
    }

    return None;
}

pub fn get_float_number_optional(prop_a: &serde_json::Value) -> Option<f64> {
    if let Some(v) = prop_a.as_f64() {
        return Some(v);
    }

    if let Some(v) = prop_a.as_i64() {
        if let Ok(v) = i32::try_from(v) {
            return Some(f64::from(v));
        }
    }

    if let Some(v) = prop_a.as_str() {
        if let Ok(v) = f64::from_str(v) {
            return Some(v);
        }
    }

    return None;
}

pub fn get_i64_optional(prop_a: &serde_json::Value) -> Option<i64> {
    if let Some(v) = prop_a.as_i64() {
        return Some(v);
    }

    if let Some(v) = prop_a.as_str() {
        if let Ok(v) = i64::from_str(v) {
            return Some(v);
        }
    }

    return None;
}

pub fn get_i16_array_optional(prop_a: &serde_json::Value) -> Option<Vec<Option<i16>>> {
    // O esperado é que a propriedade seja um array
    if let Some(vec) = prop_a.as_array() {
        let mut arr: Vec<Option<i16>> = Vec::with_capacity(vec.len());
        for prop_a in vec {
            let v = get_i16_number_optional(prop_a);
            arr.push(v);
        }
        return Some(arr);
    }

    // Se não for array e for um valor numérico, a gente transforma em array com 1 valor
    if let Some(v) = get_i16_number_optional(prop_a) {
        return Some(vec![Some(v)]);
    }

    return None;
}

pub fn get_i16_number_optional(prop_a: &serde_json::Value) -> Option<i16> {
    if let Some(v) = prop_a.as_i64() {
        if let Ok(v) = i16::try_from(v) {
            return Some(v);
        }
    }

    if let Some(v) = prop_a.as_str() {
        if let Ok(v) = i16::from_str(v) {
            return Some(v);
        }
    }

    return None;
}

pub fn get_int_number_array_prop(prop_a: &serde_json::Value) -> Result<Vec<Option<i16>>, String> {
    // let prop_a = match prop_o { Some(a) => a, None => return Ok(vec![None;array_length]) };
    if let Some(vec) = prop_a.as_array() {
        let mut arr = Vec::<Option<i16>>::new();
        arr.reserve(vec.len());
        for prop_a in vec {
            if let Some(v) = prop_a.as_i64() {
                arr.push(Some(i16::try_from(v).unwrap()));
            } else if let Some(v) = prop_a.as_str() {
                arr.push(match i16::from_str(v) {
                    Ok(v) => Some(v),
                    Err(_err) => None,
                });
            } else {
                arr.push(None);
            }
        }
        return Ok(arr);
    } else if let Some(vec_str) = prop_a.as_str() {
        let vec: serde_json::Value =
            serde_json::from_str(vec_str).map_err(|e| format!("EROR40 {}", e))?;
        if let Some(vec) = vec.as_array() {
            let mut arr = Vec::<Option<i16>>::new();
            arr.reserve(vec.len());
            for prop_a in vec {
                if let Some(v) = prop_a.as_i64() {
                    arr.push(Some(i16::try_from(v).unwrap()));
                } else {
                    arr.push(None);
                }
            }
            return Ok(arr);
        }
    }
    return Err("Could not find valid attribute value".to_owned());
}

pub fn get_bool_array_prop(
    prop_o: &Option<&serde_json::Value>,
) -> Result<Vec<Option<bool>>, String> {
    let prop_a = match prop_o {
        Some(a) => a,
        None => return Err("Could not find valid attribute value".to_owned()),
    };
    return get_bool_array_prop_2(prop_a);
}
pub fn get_bool_array_prop_2(prop_a: &serde_json::Value) -> Result<Vec<Option<bool>>, String> {
    if let Some(vec) = prop_a.as_array() {
        let mut arr = Vec::<Option<bool>>::new();
        arr.reserve(vec.len());
        for prop_a in vec {
            if let Some(v) = prop_a.as_bool() {
                arr.push(Some(v));
            } else if let Some(val_n) = prop_a.as_i64() {
                if val_n == 1 {
                    arr.push(Some(true));
                } else if val_n == 0 {
                    arr.push(Some(false));
                } else {
                    arr.push(None);
                }
            } else {
                arr.push(None);
            }
        }
        return Ok(arr);
    }
    return Err("Could not find valid attribute value".to_owned());
}

pub fn get_bool_optional(prop_a: &serde_json::Value) -> Option<bool> {
    if let Some(v) = prop_a.as_bool() {
        return Some(v);
    }

    if let Some(v) = prop_a.as_i64() {
        if v == 0 {
            return Some(false);
        }
        if v == 1 {
            return Some(true);
        }
    }

    return None;
}

pub fn get_bool_array_optional(prop_a: &serde_json::Value) -> Option<Vec<Option<bool>>> {
    // O esperado é que a propriedade seja um array
    if let Some(vec) = prop_a.as_array() {
        let mut arr: Vec<Option<bool>> = Vec::with_capacity(vec.len());
        for prop_a in vec {
            let v = get_bool_optional(prop_a);
            arr.push(v);
        }
        return Some(arr);
    }

    // Se não for array e for um valor numérico, a gente transforma em array com 1 valor
    if let Some(v) = get_bool_optional(prop_a) {
        return Some(vec![Some(v)]);
    }

    return None;
}

pub fn get_string_prop(prop_o: &Option<&serde_json::Value>) -> Result<String, String> {
    match prop_o {
        None => return Err("Attribute is empty".to_owned()),
        Some(prop_a) => match prop_a.as_str() {
            Some(value) => return Ok(value.to_owned()),
            None => return Err("Could not find valid attribute value".to_owned()),
        },
    };
}

pub fn get_int_number_prop(prop_o: &Option<&serde_json::Value>) -> Result<i64, String> {
    match prop_o {
        None => return Err("Attribute is empty".to_owned()),
        Some(prop_a) => match prop_a.as_i64() {
            Some(value) => return Ok(value),
            None => return Err("Could not find valid attribute value".to_owned()),
        },
    };
}

pub fn get_string_array_prop(prop_o: &Option<&serde_json::Value>) -> Result<Vec<String>, String> {
    let prop_a = match prop_o {
        Some(a) => a,
        None => return Err("Could not find valid attribute value".to_owned()),
    };
    if let Some(vec) = prop_a.as_array() {
        let mut arr = Vec::<String>::new();
        arr.reserve(vec.len());
        for prop_a in vec {
            if let Some(v) = prop_a.as_str() {
                arr.push(v.to_string());
            }
        }
        return Ok(arr);
    } else if let Some(value) = prop_a.as_str() {
        let mut arr = Vec::<String>::new();
        arr.push(value.to_string());
        return Ok(arr);
    }
    return Err("Could not find valid attribute value".to_owned());
}

pub fn get_bool_prop(prop_o: &Option<&serde_json::Value>) -> Result<bool, String> {
    match prop_o {
        None => Err("Attribute is empty".to_owned()),
        Some(prop_a) => match prop_a.as_bool() {
            Some(value) => Ok(value),
            None => Err("Could not find valid attribute value".to_owned()),
        },
    }
}
