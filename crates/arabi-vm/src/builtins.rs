use crate::frame::{Value, SharedList, SharedDict, SharedSet, ExceptionData, RangeData};
use crate::error::RuntimeError;
use std::rc::Rc;
use std::io::Read;
use std::path::Path;

pub fn read_source_file(path: &Path) -> Result<String, std::io::Error> {
    let raw = std::fs::read(path)?;
    if raw.len() >= 2 && raw[0] == 0xFF && raw[1] == 0xFE {
        Ok(String::from_utf16_lossy(
            &raw[2..].chunks(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect::<Vec<_>>()
        ))
    } else if raw.len() >= 2 && raw[0] == 0xFE && raw[1] == 0xFF {
        Ok(String::from_utf16_lossy(
            &raw[2..].chunks(2).map(|c| u16::from_be_bytes([c[0], c[1]])).collect::<Vec<_>>()
        ))
    } else if raw.len() >= 3 && raw[0] == 0xEF && raw[1] == 0xBB && raw[2] == 0xBF {
        Ok(String::from_utf8_lossy(&raw[3..]).into_owned())
    } else {
        Ok(String::from_utf8_lossy(&raw).into_owned())
    }
}

fn vm_to_string(val: &Value, vm: &mut crate::vm::VM, module: &mut arabi_compiler::bytecode::BytecodeModule) -> String {
    if let Value::Instance(rc) = val {
        let display_clone = rc.class.methods.get("__عرض__").cloned();
        let str_clone = rc.class.methods.get("__نص__").cloned();
        if let Some(method) = display_clone {
            if let Ok(result) = method.call(std::slice::from_ref(val), &[], vm, module) {
                return result.to_string_value();
            }
        }
        if let Some(method) = str_clone {
            if let Ok(result) = method.call(std::slice::from_ref(val), &[], vm, module) {
                return result.to_string_value();
            }
        }
    }
    val.to_string_value()
}

#[cfg(feature = "random")]
fn rand_random() -> f64 {
    use rand::Rng;
    rand::thread_rng().gen::<f64>()
}

#[cfg(not(feature = "random"))]
fn rand_random() -> f64 {
    static SEED: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0x1234567890ABCDEF);
    let old = SEED.fetch_add(6364136223846793005, std::sync::atomic::Ordering::Relaxed);
    (old >> 11) as f64 / (1u64 << 53) as f64
}

pub fn call_native(name: &str, args: &[Value], kwargs: &[(String, Value)], vm: &mut crate::vm::VM, module: &mut arabi_compiler::bytecode::BytecodeModule) -> Result<Value, RuntimeError> {
    match name {
        "اطبع" => {
            let strs: Vec<String> = args.iter().map(|a| vm_to_string(a, vm, module)).collect();
            let mut separator = " ".to_string();
            let mut end = "\n".to_string();
            let mut flush = false;
            for (k, v) in kwargs {
                match k.as_str() {
                    "الفاصل" => { separator = v.to_string_value(); }
                    "النهاية" => { end = v.to_string_value().replace("\\س", "\n"); }
                    "مباشر" => { flush = v.is_truthy(); }
                    _ => {}
                }
            }
            let output = strs.join(&separator);
            print!("{}{}", output, end);
            if flush {
                use std::io::Write;
                let _ = std::io::stdout().flush();
            }
            Ok(Value::Null)
        }
        "ادخل" => {
            let prompt = args.first().map(|a| a.to_string_value()).unwrap_or_default();
            print!("{}", prompt);
            use std::io::Write;
            let _ = std::io::stdout().flush();
            let mut input = String::new();
            if std::io::stdin().read_line(&mut input).is_err() {
                return Err("فشل في قراءة المدخلات".into());
            }
            Ok(Value::String(input.trim().to_string().into()))
        }
        "طول" => {
            let obj = args.first().ok_or_else(|| RuntimeError::new("طول يتطلب معامل واحد"))?;
            match obj {
                Value::List(items) => Ok(Value::Integer(items.borrow().len() as i64)),
                Value::String(s) => Ok(Value::Integer(if s.is_ascii() { s.len() } else { s.chars().count() } as i64)),
                Value::Tuple(items) => Ok(Value::Integer(items.len() as i64)),
                Value::Dict(pairs) => Ok(Value::Integer(pairs.borrow().len() as i64)),
                Value::Set(items) => Ok(Value::Integer(items.borrow().len() as i64)),
                Value::Instance(rc) => {
                    let method_clone = rc.class.methods.get("__طول__").cloned();
                    if let Some(method) = method_clone {
                        match method.call(std::slice::from_ref(obj), &[], vm, module) {
                            Ok(Value::Integer(n)) => Ok(Value::Integer(n)),
                            _ => Err(RuntimeError::new("طول يتطلب عدداً صحيحاً من __طول__")),
                        }
                    } else {
                        Err(RuntimeError::new("طول غير مدعوم لهذا النوع"))
                    }
                }
                _ => Err(RuntimeError::new_typed("استثناء_نوع", "طول غير مدعوم لهذا النوع")),
            }
        }
        "مصفوفة" => {
            let obj = args.first().ok_or_else(|| RuntimeError::new("مصفوفة تتطلب معامل واحد"))?;
            match obj {
                Value::Tuple(items) => Ok(Value::List(SharedList::new(items.as_ref().clone()))),
                Value::List(items) => Ok(Value::List(items.clone())),
                Value::String(s) => {
                    let chars: Vec<Value> = s.chars().map(|c| Value::String(Rc::new(c.to_string()))).collect();
                    Ok(Value::List(SharedList::new(chars)))
                }
                Value::Set(items) => Ok(Value::List(SharedList::new(items.borrow().clone()))),
                Value::Range(d) => {
                    let mut result = Vec::new();
                    let mut i = d.start;
                    while i < d.end {
                        result.push(Value::Integer(i));
                        i += d.step;
                    }
                    Ok(Value::List(SharedList::new(result)))
                }
                _ => Err(RuntimeError::new_typed("استثناء_نوع", "غير قادر على التحويل لمصفوفة")),
            }
        }
        "قائمة_مليئة" => {
            if args.len() < 2 {
                return Err(RuntimeError::new_typed("استثناء_نوع", "قائمة_مليئة تتطلب حجم وقيمة"));
            }
            let size = match &args[0] {
                Value::Integer(n) => *n as usize,
                _ => return Err(RuntimeError::new_typed("استثناء_نوع", "قائمة_مليئة يتطلب حجم صحيح")),
            };
            let val = args[1].clone();
            let mut items = Vec::with_capacity(size);
            items.resize(size, val);
            Ok(Value::List(SharedList::new(items)))
        }
        "مترابطة" => {
            let obj = args.first().ok_or_else(|| RuntimeError::new("مترابطة تتطلب معامل واحد"))?;
            match obj {
                Value::List(items) => Ok(Value::Tuple(Rc::new(items.borrow().clone()))),
                Value::String(s) => {
                    let chars: Vec<Value> = s.chars().map(|c| Value::String(Rc::new(c.to_string()))).collect();
                    Ok(Value::Tuple(Rc::new(chars)))
                }
                _ => Err(RuntimeError::new_typed("استثناء_نوع", "غير قادر على التحويل لمترابطة")),
            }
        }
        "صحيح" => {
            let obj = args.first().ok_or_else(|| RuntimeError::new("صحيح يتطلب معامل واحد"))?;
            match obj {
                Value::Integer(_) => Ok(obj.clone()),
                Value::Float(f) => Ok(Value::Integer(*f as i64)),
                Value::String(s) => {
                    let n: i64 = s.parse().map_err(|_| RuntimeError::new_typed("استثناء_نوع", "غير قادر على التحويل لصحيح"))?;
                    Ok(Value::Integer(n))
                }
                Value::Boolean(b) => Ok(Value::Integer(if *b { 1 } else { 0 })),
                Value::Null => Ok(Value::Integer(0)),
                _ => Ok(Value::Integer(1)),
            }
        }
        "عشري" => {
            let obj = args.first().ok_or_else(|| RuntimeError::new("عشري يتطلب معامل واحد"))?;
            match obj {
                Value::Float(_) => Ok(obj.clone()),
                Value::Integer(n) => Ok(Value::Float(*n as f64)),
                Value::String(s) => {
                    let f: f64 = s.parse().map_err(|_| RuntimeError::new_typed("استثناء_نوع", "غير قادر على التحويل لعشري"))?;
                    Ok(Value::Float(f))
                }
                _ => Err(RuntimeError::new_typed("استثناء_نوع", "غير قادر على التحويل لعشري")),
            }
        }
        "منطق" => {
            let obj = args.first().ok_or_else(|| RuntimeError::new("منطق يتطلب معامل واحد"))?;
            Ok(Value::Boolean(obj.is_truthy()))
        }
        "نص" => {
            let obj = args.first().ok_or_else(|| RuntimeError::new("نص يتطلب معامل واحد"))?;
            Ok(Value::String(obj.to_string_value().into()))
        }
        "مميزة" => {
            let obj = args.first().ok_or_else(|| RuntimeError::new("مميزة تتطلب معامل واحد"))?;
            match obj {
                Value::List(items) => {
                    let borrow = items.borrow();
                    let mut unique = Vec::new();
                    for item in borrow.iter() {
                        if !unique.contains(item) {
                            unique.push(item.clone());
                        }
                    }
                    Ok(Value::Set(SharedSet::new(unique)))
                }
                Value::Tuple(items) => {
                    let mut unique = Vec::new();
                    for item in items.iter() {
                        if !unique.contains(item) {
                            unique.push(item.clone());
                        }
                    }
                    Ok(Value::Set(SharedSet::new(unique)))
                }
                _ => Err(RuntimeError::new_typed("استثناء_نوع", "غير قادر على التحويل لمميزة")),
            }
        }
        "مجموع" => {
            let obj = args.first().ok_or_else(|| RuntimeError::new("مجموع يتطلب معامل واحد"))?;
            match obj {
                Value::List(items) => {
                    let mut sum = 0.0;
                    for item in items.borrow().iter() {
                        match item {
                            Value::Integer(n) => sum += *n as f64,
                            Value::Float(f) => sum += *f,
                            _ => return Err(RuntimeError::new_typed("استثناء_نوع", "مجموع غير مدعوم لهذا النوع")),
                        }
                    }
                    if sum == sum.floor() {
                        Ok(Value::Integer(sum as i64))
                    } else {
                        Ok(Value::Float(sum))
                    }
                }
                Value::Tuple(items) => {
                    let mut sum = 0.0;
                    for item in items.iter() {
                        match item {
                            Value::Integer(n) => sum += *n as f64,
                            Value::Float(f) => sum += *f,
                            _ => return Err(RuntimeError::new_typed("استثناء_نوع", "مجموع غير مدعوم لهذا النوع")),
                        }
                    }
                    if sum == sum.floor() {
                        Ok(Value::Integer(sum as i64))
                    } else {
                        Ok(Value::Float(sum))
                    }
                }
                _ => Err(RuntimeError::new_typed("استثناء_نوع", "مجموع يتطلب قائمة")),
            }
        }
        "اكبر" => {
            if args.len() == 1 {
                match &args[0] {
                    Value::List(items) => {
                        let borrow = items.borrow();
                        let mut max = borrow.first().ok_or_else(|| RuntimeError::new("القائمة فارغة"))?.clone();
                        for item in borrow.iter().skip(1) {
                            if item.partial_cmp(&max) == Some(std::cmp::Ordering::Greater) {
                                max = item.clone();
                            }
                        }
                        Ok(max)
                    }
                    Value::Tuple(items) => {
                        let mut max = items.first().ok_or_else(|| RuntimeError::new("القائمة فارغة"))?.clone();
                        for item in items.iter().skip(1) {
                            if item.partial_cmp(&max) == Some(std::cmp::Ordering::Greater) {
                                max = item.clone();
                            }
                        }
                        Ok(max)
                    }
                    _ => Err(RuntimeError::new_typed("استثناء_نوع", "اكبر يتطلب قائمة")),
                }
            } else if args.len() == 2 {
                let items = &args[0];
                let key_func = &args[1];
                let values: Vec<Value> = match items {
                    Value::List(l) => l.borrow().clone(),
                    Value::Tuple(t) => t.as_ref().clone(),
                    _ => return Err(RuntimeError::new_typed("استثناء_نوع", "اكبر يتطلب قائمة")),
                };
                let mut max_item = values.first().ok_or_else(|| RuntimeError::new("القائمة فارغة"))?.clone();
                let mut max_key = key_func.call(&[max_item.clone()], &[], vm, module)?;
                for item in values.iter().skip(1) {
                    let k = key_func.call(&[item.clone()], &[], vm, module)?;
                    if k.partial_cmp(&max_key) == Some(std::cmp::Ordering::Greater) {
                        max_key = k;
                        max_item = item.clone();
                    }
                }
                Ok(max_item)
            } else {
                let mut max = args[0].clone();
                for item in args.iter().skip(1) {
                    if item.partial_cmp(&max) == Some(std::cmp::Ordering::Greater) {
                        max = item.clone();
                    }
                }
                Ok(max)
            }
        }
        "اصغر" => {
            if args.len() == 1 {
                match &args[0] {
                    Value::List(items) => {
                        let borrow = items.borrow();
                        let mut min = borrow.first().ok_or_else(|| RuntimeError::new("القائمة فارغة"))?.clone();
                        for item in borrow.iter().skip(1) {
                            if item.partial_cmp(&min) == Some(std::cmp::Ordering::Less) {
                                min = item.clone();
                            }
                        }
                        Ok(min)
                    }
                    Value::Tuple(items) => {
                        let mut min = items.first().ok_or_else(|| RuntimeError::new("القائمة فارغة"))?.clone();
                        for item in items.iter().skip(1) {
                            if item.partial_cmp(&min) == Some(std::cmp::Ordering::Less) {
                                min = item.clone();
                            }
                        }
                        Ok(min)
                    }
                    _ => Err(RuntimeError::new_typed("استثناء_نوع", "اصغر يتطلب قائمة")),
                }
            } else if args.len() == 2 {
                let items = &args[0];
                let key_func = &args[1];
                let values: Vec<Value> = match items {
                    Value::List(l) => l.borrow().clone(),
                    Value::Tuple(t) => t.as_ref().clone(),
                    _ => return Err(RuntimeError::new_typed("استثناء_نوع", "اصغر يتطلب قائمة")),
                };
                let mut min_item = values.first().ok_or_else(|| RuntimeError::new("القائمة فارغة"))?.clone();
                let mut min_key = key_func.call(&[min_item.clone()], &[], vm, module)?;
                for item in values.iter().skip(1) {
                    let k = key_func.call(&[item.clone()], &[], vm, module)?;
                    if k.partial_cmp(&min_key) == Some(std::cmp::Ordering::Less) {
                        min_key = k;
                        min_item = item.clone();
                    }
                }
                Ok(min_item)
            } else {
                let mut min = args[0].clone();
                for item in args.iter().skip(1) {
                    if item.partial_cmp(&min) == Some(std::cmp::Ordering::Less) {
                        min = item.clone();
                    }
                }
                Ok(min)
            }
        }
        "مدى" => {
            match args.len() {
                1 => {
                    let end = match &args[0] {
                        Value::Integer(n) => *n,
                        _ => return Err(RuntimeError::new_typed("استثناء_نوع", "النطاق يتطلب اعداد صحيحة")),
                    };
                    Ok(Value::Range(Box::new(RangeData { start: 0, end, step: 1 })))
                }
                2 => {
                    let start = match &args[0] {
                        Value::Integer(n) => *n,
                        _ => return Err(RuntimeError::new_typed("استثناء_نوع", "النطاق يتطلب اعداد صحيحة")),
                    };
                    let end = match &args[1] {
                        Value::Integer(n) => *n,
                        _ => return Err(RuntimeError::new_typed("استثناء_نوع", "النطاق يتطلب اعداد صحيحة")),
                    };
                    Ok(Value::Range(Box::new(RangeData { start, end, step: 1 })))
                }
                3 => {
                    let start = match &args[0] {
                        Value::Integer(n) => *n,
                        _ => return Err(RuntimeError::new_typed("استثناء_نوع", "النطاق يتطلب اعداد صحيحة")),
                    };
                    let end = match &args[1] {
                        Value::Integer(n) => *n,
                        _ => return Err(RuntimeError::new_typed("استثناء_نوع", "النطاق يتطلب اعداد صحيحة")),
                    };
                    let step = match &args[2] {
                        Value::Integer(n) => *n,
                        _ => return Err(RuntimeError::new_typed("استثناء_نوع", "النطاق يتطلب اعداد صحيحة")),
                    };
                    if step == 0 {
                        return Err(RuntimeError::new_typed("استثناء_قيمة", "خطوة النطاق لا يمكن ان تكون صفر"));
                    }
                    Ok(Value::Range(Box::new(RangeData { start, end, step })))
                }
                _ => Err(RuntimeError::new_typed("استثناء_نوع", "النطاق يتطلب 1-3 معاملات")),
            }
        }
        "مقرون" => {
            let mut lists: Vec<Vec<Value>> = Vec::new();
            for arg in args {
                match arg {
                    Value::List(items) => lists.push(items.borrow().clone()),
                    Value::Tuple(items) => lists.push(items.as_ref().clone()),
                    _ => return Err(RuntimeError::new_typed("استثناء_نوع", "مقرون يتطلب قوائم")),
                }
            }
            if lists.is_empty() {
                return Ok(Value::List(SharedList::new(Vec::new())));
            }
            let min_len = lists.iter().map(|l| l.len()).min().unwrap_or(0);
            let mut result = Vec::new();
            for i in 0..min_len {
                let tuple: Vec<Value> = lists.iter().map(|l| l[i].clone()).collect();
                result.push(Value::Tuple(Rc::new(tuple)));
            }
            Ok(Value::List(SharedList::new(result)))
        }
        "هل_نوع" => {
            if args.len() < 2 {
                return Err(RuntimeError::new("هل_نوع يتطلب معاملين"));
            }
            let type_name = match &args[1] {
                Value::String(s) => s.as_str(),
                _ => return Err(RuntimeError::new("المعامل الثاني يجب ان يكون نصاً")),
            };
            let is_type = match type_name {
                "صحيح" => matches!(&args[0], Value::Integer(_)),
                "عشري" => matches!(&args[0], Value::Float(_)),
                "نص" => matches!(&args[0], Value::String(_)),
                "منطق" => matches!(&args[0], Value::Boolean(_)),
                "عدم" => matches!(&args[0], Value::Null),
                "مصفوفة" => matches!(&args[0], Value::List(_)),
                "مترابطة" => matches!(&args[0], Value::Tuple(_)),
                "فهرس" => matches!(&args[0], Value::Dict(_)),
                "مميزة" => matches!(&args[0], Value::Set(_)),
                _ => false,
            };
            Ok(Value::Boolean(is_type))
        }
        "نوع" => {
            let obj = args.first().ok_or_else(|| RuntimeError::new("نوع يتطلب معامل واحد"))?;
            Ok(Value::String(obj.type_name().to_string().into()))
        }
        "صحيح؟" => {
            let obj = args.first().ok_or_else(|| RuntimeError::new("صحيح؟ يتطلب معامل واحد"))?;
            Ok(Value::Boolean(matches!(obj, Value::Integer(_))))
        }
        "عشري؟" => {
            let obj = args.first().ok_or_else(|| RuntimeError::new("عشري؟ يتطلب معامل واحد"))?;
            Ok(Value::Boolean(matches!(obj, Value::Float(_))))
        }
        "نص؟" => {
            let obj = args.first().ok_or_else(|| RuntimeError::new("نص؟ يتطلب معامل واحد"))?;
            Ok(Value::Boolean(matches!(obj, Value::String(_))))
        }
        "قائمة؟" => {
            let obj = args.first().ok_or_else(|| RuntimeError::new("قائمة؟ يتطلب معامل واحد"))?;
            Ok(Value::Boolean(matches!(obj, Value::List(_))))
        }
        "معكوس" => {
            let obj = args.first().ok_or_else(|| RuntimeError::new("معكوس يتطلب معامل واحد"))?;
            match obj {
                Value::List(items) => {
                    let mut reversed = items.borrow().clone();
                    reversed.reverse();
                    Ok(Value::List(SharedList::new(reversed)))
                }
                Value::String(s) => {
                    let reversed: String = s.chars().rev().collect();
                    Ok(Value::String(reversed.into()))
                }
                _ => Err(RuntimeError::new_typed("استثناء_نوع", "معكوس غير مدعوم لهذا النوع")),
            }
        }
        "تحقق_اي" => {
            let obj = args.first().ok_or_else(|| RuntimeError::new("تحقق_اي يتطلب معامل واحد"))?;
            match obj {
                Value::List(items) => {
                    Ok(Value::Boolean(items.borrow().iter().all(|v| v.is_truthy())))
                }
                Value::Tuple(items) => {
                    Ok(Value::Boolean(items.iter().all(|v| v.is_truthy())))
                }
                _ => Err(RuntimeError::new_typed("استثناء_نوع", "تحقق_اي يتطلب قائمة")),
            }
        }
        "تحقق_او" => {
            let obj = args.first().ok_or_else(|| RuntimeError::new("تحقق_او يتطلب معامل واحد"))?;
            match obj {
                Value::List(items) => {
                    Ok(Value::Boolean(items.borrow().iter().any(|v| v.is_truthy())))
                }
                Value::Tuple(items) => {
                    Ok(Value::Boolean(items.iter().any(|v| v.is_truthy())))
                }
                _ => Err(RuntimeError::new_typed("استثناء_نوع", "تحقق_او يتطلب قائمة")),
            }
        }
        "معدل" => {
            let obj = args.first().ok_or_else(|| RuntimeError::new("معدل يتطلب معامل واحد"))?;
            match obj {
                Value::List(items) => {
                    let borrow = items.borrow();
                    if borrow.is_empty() {
                        return Ok(Value::Float(0.0));
                    }
                    let mut sum = 0.0;
                    for item in borrow.iter() {
                        match item {
                            Value::Integer(n) => sum += *n as f64,
                            Value::Float(f) => sum += *f,
                            _ => return Err(RuntimeError::new_typed("استثناء_نوع", "معدل غير مدعوم لهذا النوع")),
                        }
                    }
                    Ok(Value::Float(sum / borrow.len() as f64))
                }
                Value::Tuple(items) => {
                    if items.is_empty() {
                        return Ok(Value::Float(0.0));
                    }
                    let mut sum = 0.0;
                    for item in items.iter() {
                        match item {
                            Value::Integer(n) => sum += *n as f64,
                            Value::Float(f) => sum += *f,
                            _ => return Err(RuntimeError::new_typed("استثناء_نوع", "معدل غير مدعوم لهذا النوع")),
                        }
                    }
                    Ok(Value::Float(sum / items.len() as f64))
                }
                _ => Err(RuntimeError::new_typed("استثناء_نوع", "معدل يتطلب قائمة")),
            }
        }
        "وقت" => {
            use std::time::SystemTime;
            let now = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default();
            let secs = now.as_secs();
            let millis = now.subsec_millis() as u64;
            let total_millis = secs * 1000 + millis;
            let dict = vec![
                (Value::String(Rc::new("الثوان".to_string())), Value::Float(secs as f64)),
                (Value::String(Rc::new("الملي_ثانية".to_string())), Value::Integer(millis as i64)),
                (Value::String(Rc::new("الكل".to_string())), Value::Float(total_millis as f64 / 1000.0)),
            ];
            Ok(Value::Dict(SharedDict::new(dict)))
        }
        "الوقت" => {
            use std::time::SystemTime;
            let t = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs_f64();
            Ok(Value::Float(t))
        }
        "عد_تلقائي" => {
            use std::time::SystemTime;
            let t = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as i64;
            Ok(Value::Integer(t))
        }

        "جيب" => {
            let x = get_float_arg(args, 0)?;
            Ok(Value::Float(x.to_radians().sin()))
        }
        "تجيب" => {
            let x = get_float_arg(args, 0)?;
            Ok(Value::Float(x.to_radians().cos()))
        }
        "ظل" => {
            let x = get_float_arg(args, 0)?;
            Ok(Value::Float(x.to_radians().tan()))
        }
        "جيب_عكسي" => {
            let x = get_float_arg(args, 0)?;
            if !(-1.0..=1.0).contains(&x) {
                return Err(RuntimeError::new_typed("استثناء_قيمة", "القيمة يجب ان تكون بين -1 و 1"));
            }
            Ok(Value::Float(x.asin().to_degrees()))
        }
        "تجيب_عكسي" => {
            let x = get_float_arg(args, 0)?;
            if !(-1.0..=1.0).contains(&x) {
                return Err(RuntimeError::new_typed("استثناء_قيمة", "القيمة يجب ان تكون بين -1 و 1"));
            }
            Ok(Value::Float(x.acos().to_degrees()))
        }
        "ظل_عكسي" => {
            let x = get_float_arg(args, 0)?;
            Ok(Value::Float(x.atan().to_degrees()))
        }
        "جذر" | "مربع" => {
            let x = get_float_arg(args, 0)?;
            if x < 0.0 {
                return Err(RuntimeError::new_typed("استثناء_قيمة", "لا يمكن جذر مربع لرقم سالب"));
            }
            Ok(Value::Float(x.sqrt()))
        }
        "جذر_مكعب" | "مكعب" => {
            let x = get_float_arg(args, 0)?;
            Ok(Value::Float(x.cbrt()))
        }
        "قوة" => {
            if args.len() < 2 {
                return Err(RuntimeError::new("قوة تتطلب معاملين"));
            }
            let base = get_float_arg(args, 0)?;
            let exp = get_float_arg(args, 1)?;
            Ok(Value::Float(base.powf(exp)))
        }
        "مطلق" | "قيمة_مطلقة" => {
            let x = args.first().ok_or_else(|| RuntimeError::new("مطلق يتطلب معامل واحد"))?;
            match x {
                Value::Integer(n) => Ok(Value::Integer(n.abs())),
                Value::Float(f) => Ok(Value::Float(f.abs())),
                _ => {
                    let f = get_float_arg(args, 0)?;
                    Ok(Value::Float(f.abs()))
                }
            }
        }
        "ارضية" => {
            let x = get_float_arg(args, 0)?;
            Ok(Value::Float(x.floor()))
        }
        "سقف" => {
            let x = get_float_arg(args, 0)?;
            Ok(Value::Float(x.ceil()))
        }
        "تقريب" => {
            let x = get_float_arg(args, 0)?;
            let decimals = get_optional_int_arg(args, 1).unwrap_or(0) as u32;
            let factor = 10.0_f64.powi(decimals as i32);
            let result = (x * factor).round() / factor;
            if decimals == 0 {
                Ok(Value::Integer(result as i64))
            } else {
                Ok(Value::Float(result))
            }
        }
        "لوغ" => {
            let x = get_float_arg(args, 0)?;
            if x <= 0.0 {
                return Err(RuntimeError::new_typed("استثناء_قيمة", "اللوغاريتم الطبيعي يتطلب رقم موجب"));
            }
            Ok(Value::Float(x.ln()))
        }
        "لوغ10" => {
            let x = get_float_arg(args, 0)?;
            if x <= 0.0 {
                return Err(RuntimeError::new_typed("استثناء_قيمة", "اللوغاريتم العشري يتطلب رقم موجب"));
            }
            Ok(Value::Float(x.log10()))
        }
        "لوغ2" => {
            let x = get_float_arg(args, 0)?;
            if x <= 0.0 {
                return Err(RuntimeError::new_typed("استثناء_قيمة", "اللوغاريتم الثنائي يتطلب رقم موجب"));
            }
            Ok(Value::Float(x.log2()))
        }
        "أس" => {
            let x = get_float_arg(args, 0)?;
            Ok(Value::Float(x.exp()))
        }
        "أس2" => {
            let x = get_float_arg(args, 0)?;
            Ok(Value::Float(2.0_f64.powf(x)))
        }
        "أس10" => {
            let x = get_float_arg(args, 0)?;
            Ok(Value::Float(10.0_f64.powf(x)))
        }
        "جذر_ثلاثي" => {
            let x = get_float_arg(args, 0)?;
            Ok(Value::Float(x.cbrt()))
        }
        "سلب" => {
            let x = args.first().ok_or_else(|| RuntimeError::new("سلب يتطلب معامل واحد"))?;
            match x {
                Value::Integer(n) => Ok(Value::Integer(-n)),
                Value::Float(f) => Ok(Value::Float(-f)),
                _ => {
                    let f = get_float_arg(args, 0)?;
                    Ok(Value::Float(-f))
                }
            }
        }
        "صفر" => Ok(Value::Boolean(false)),
        "صحيح_قيمة" => Ok(Value::Boolean(true)),
        "لاشي" => Ok(Value::Null),
        "ط" => Ok(Value::Float(std::f64::consts::PI)),
        "ه" => Ok(Value::Float(std::f64::consts::E)),
        "ن" => Ok(Value::Float(std::f64::consts::TAU)),
        "مضروب" => {
            let n = get_float_arg(args, 0)?;
            if n < 0.0 || n != n.floor() {
                return Err(RuntimeError::new_typed("استثناء_قيمة", "المضروب يتطلب عدداً طبيعياً غير سالب"));
            }
            let mut result: f64 = 1.0;
            for i in 2..=n as u64 {
                result *= i as f64;
            }
            Ok(Value::Float(result))
        }
        "قم_اكبر" => {
            let y = get_float_arg(args, 0)?;
            let x = get_float_arg(args, 1)?;
            Ok(Value::Float(y.atan2(x)))
        }
        "حد_اعلى" => {
            let x = get_float_arg(args, 0)?;
            Ok(Value::Float(x.trunc()))
        }
        "حد_ادنى" => {
            let x = get_float_arg(args, 0)?;
            Ok(Value::Float(x.floor()))
        }
        "راديان" => {
            let x = get_float_arg(args, 0)?;
            Ok(Value::Float(x.to_radians()))
        }
        "درجة" => {
            let x = get_float_arg(args, 0)?;
            Ok(Value::Float(x.to_degrees()))
        }
        "قسمة_ومظم" => {
            if args.len() < 2 { return Err(RuntimeError::new("قسمة_ومعظم يتطلب معاملين")); }
            let a = match &args[0] {
                Value::Integer(n) => *n as f64,
                Value::Float(f) => *f,
                _ => return Err(RuntimeError::new("المعامل الاول يجب ان يكون عدداً")),
            };
            let b = match &args[1] {
                Value::Integer(n) => *n as f64,
                Value::Float(f) => *f,
                _ => return Err(RuntimeError::new("المعامل الثاني يجب ان يكون عدداً")),
            };
            if b == 0.0 { return Err(RuntimeError::new_typed("استثناء_قسمة", "القسمة على صفر")); }
            Ok(Value::List(SharedList::new(vec![
                Value::Float((a / b).floor()),
                Value::Float(a % b),
            ])))
        }
        "مغلق" => {
            if args.len() < 2 { return Err(RuntimeError::new("مغلق يتطلب معاملين")); }
            let a = match &args[0] {
                Value::Integer(n) => *n as f64,
                Value::Float(f) => *f,
                _ => return Err(RuntimeError::new("المعامل الاول يجب ان يكون عدداً")),
            };
            let b = match &args[1] {
                Value::Integer(n) => *n as f64,
                Value::Float(f) => *f,
                _ => return Err(RuntimeError::new("المعامل الثاني يجب ان يكون عدداً")),
            };
            let rel_tol = args.get(2).and_then(|v| match v {
                Value::Float(f) => Some(*f), Value::Integer(n) => Some(*n as f64), _ => None,
            }).unwrap_or(1e-9);
            let abs_tol = args.get(3).and_then(|v| match v {
                Value::Float(f) => Some(*f), Value::Integer(n) => Some(*n as f64), _ => None,
            }).unwrap_or(0.0);
            let diff = (a - b).abs();
            let largest = if a.abs() > b.abs() { a.abs() } else { b.abs() };
            Ok(Value::Boolean(diff <= abs_tol + rel_tol * largest))
        }
        "لانهاية" => {
            let x = get_float_arg(args, 0)?;
            Ok(Value::Boolean(x.is_infinite()))
        }
        "ليس_رقم" => {
            let x = get_float_arg(args, 0)?;
            Ok(Value::Boolean(x.is_nan()))
        }
        "مجموع_مربعات" => {
            let obj = args.first().ok_or_else(|| RuntimeError::new("مجموع_مربعات يتطلب مصفوفة"))?;
            match obj {
                Value::List(list) => {
                    let items = list.borrow();
                    let sum: f64 = items.iter().map(|v| {
                        let x = match v { Value::Integer(n) => *n as f64, Value::Float(f) => *f, _ => 0.0 };
                        x * x
                    }).sum();
                    Ok(Value::Float(sum))
                }
                _ => Err(RuntimeError::new_typed("استثناء_نوع", "مجموع_مربعات يتطلب مصفوفة")),
            }
        }
        "متوسط_وزني" => {
            if args.len() < 2 { return Err(RuntimeError::new("متوسط_وزني يتطلب قيمتين واوزاناً")); }
            match (&args[0], &args[1]) {
                (Value::List(vals), Value::List(weights)) => {
                    let v = vals.borrow();
                    let w = weights.borrow();
                    if v.len() != w.len() { return Err(RuntimeError::new("القيم والاوزان يجب ان نفس الطول")); }
                    if v.is_empty() { return Err(RuntimeError::new("القائمة فارغة")); }
                    let mut sum_vw = 0.0;
                    let mut sum_w = 0.0;
                    for (val, wt) in v.iter().zip(w.iter()) {
                        let val_f = match val { Value::Integer(n) => *n as f64, Value::Float(f) => *f, _ => 0.0 };
                        let wt_f = match wt { Value::Integer(n) => *n as f64, Value::Float(f) => *f, _ => 0.0 };
                        sum_vw += val_f * wt_f;
                        sum_w += wt_f;
                    }
                    if sum_w == 0.0 { return Err(RuntimeError::new("مجموع الاوزان لا يمكن ان يكون صفر")); }
                    Ok(Value::Float(sum_vw / sum_w))
                }
                _ => Err(RuntimeError::new_typed("استثناء_نوع", "متوسط_وزني يتطلب مصفوفتين")),
            }
        }

        "غفوة" => {
            use std::time::Duration;
            let seconds = get_float_arg(args, 0)?;
            std::thread::sleep(Duration::from_secs_f64(seconds));
            Ok(Value::Null)
        }

        "الآن" => {
            use std::time::SystemTime;
            let t = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64;
            Ok(Value::Integer(t))
        }
        "مللي" => {
            use std::time::SystemTime;
            let t = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs_f64() * 1000.0;
            Ok(Value::Float(t))
        }
        "توقيت" => {
            use std::time::SystemTime;
            let now = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default();
            let secs = now.as_secs();
            let remaining = secs % 86400;
            let hours = remaining / 3600;
            let minutes = (remaining % 3600) / 60;
            let seconds = remaining % 60;
            Ok(Value::String(Rc::new(format!("{:02}:{:02}:{:02}", hours, minutes, seconds))))
        }
        "تاريخ" => {
            use std::time::SystemTime;
            let now = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default();
            let secs = now.as_secs();
            let days = secs / 86400;
            let year = 1970 + (days / 365) as i64;
            let month = ((days % 365) / 30 + 1) as i64;
            let day = ((days % 365) % 30 + 1) as i64;
            Ok(Value::String(Rc::new(format!("{:04}-{:02}-{:02}", year, month, day))))
        }
        "عداد" => {
            static START: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
            let start = START.get_or_init(|| std::time::Instant::now());
            Ok(Value::Float(start.elapsed().as_secs_f64()))
        }
        "زمان" => {
            use std::time::SystemTime;
            let now = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap_or_default();
            let secs = now.as_secs();
            let ms = now.subsec_millis();
            let days = secs / 86400;
            let year = 1970 + (days / 365) as i64;
            let month = ((days % 365) / 30 + 1) as i64;
            let day = ((days % 365) % 30 + 1) as i64;
            let remaining = secs % 86400;
            let hours = remaining / 3600;
            let minutes = (remaining % 3600) / 60;
            let seconds = remaining % 60;
            Ok(Value::String(Rc::new(format!("{:04}-{:02}-{:02} {:02}:{:02}:{:02}.{:03}", year, month, day, hours, minutes, seconds, ms))))
        }
        "تحويل" => {
            let s = args.first().ok_or_else(|| RuntimeError::new("تحويل يتطلب نصاً"))?;
            match s {
                Value::String(s) => {
                    Ok(Value::Float(s.parse::<f64>().unwrap_or(0.0)))
                }
                _ => Err(RuntimeError::new_typed("استثناء_نوع", "تحويل يتطلب نصاً")),
            }
        }
        "فرق" => {
            let a = args.get(0).ok_or_else(|| RuntimeError::new("فرق يتطلب تاريخين"))?;
            let b = args.get(1).ok_or_else(|| RuntimeError::new("فرق يتطلب تاريخين"))?;
            match (a, b) {
                (Value::Integer(a), Value::Integer(b)) => Ok(Value::Integer(a - b)),
                (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a - b)),
                _ => Err(RuntimeError::new_typed("استثناء_نوع", "فرق يتطلب عددين")),
            }
        }
        "سنة" => {
            use std::time::SystemTime;
            let now = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap_or_default();
            let days = now.as_secs() / 86400;
            Ok(Value::Integer(1970 + (days / 365) as i64))
        }
        "شهر" => {
            use std::time::SystemTime;
            let now = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap_or_default();
            let days = now.as_secs() / 86400;
            Ok(Value::Integer(((days % 365) / 30 + 1) as i64))
        }
        "يوم" => {
            use std::time::SystemTime;
            let now = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap_or_default();
            let days = now.as_secs() / 86400;
            Ok(Value::Integer(((days % 365) % 30 + 1) as i64))
        }
        "ساعة" => {
            use std::time::SystemTime;
            let now = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap_or_default();
            let remaining = now.as_secs() % 86400;
            Ok(Value::Integer((remaining / 3600) as i64))
        }
        "دقيقة" => {
            use std::time::SystemTime;
            let now = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap_or_default();
            let remaining = now.as_secs() % 86400;
            Ok(Value::Integer(((remaining % 3600) / 60) as i64))
        }
        "ثانية" => {
            use std::time::SystemTime;
            let now = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap_or_default();
            Ok(Value::Integer((now.as_secs() % 60) as i64))
        }
        "يوم_الاسبوع" => {
            use std::time::SystemTime;
            let now = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap_or_default();
            let days = (now.as_secs() / 86400) as i64;
            let dow = (days + 4) % 7;
            let names = ["احد", "اثنين", "ثلاثاء", "اربعاء", "خميس", "جمعة", "سبت"];
            Ok(Value::String(Rc::new(names[dow as usize].to_string())))
        }
        "هل_سنة_كبيسة" => {
            let y = args.first().ok_or_else(|| RuntimeError::new("هل_سنة_كبيسة تتطلب سنة"))?;
            let year = match y {
                Value::Integer(n) => *n,
                _ => return Err(RuntimeError::new_typed("استثناء_نوع", "هل_سنة_كبيسة تتطلب سنة صحيحة")),
            };
            Ok(Value::Boolean((year % 4 == 0 && year % 100 != 0) || year % 400 == 0))
        }
        "ايام_الشهر" => {
            let m = args.first().ok_or_else(|| RuntimeError::new("ايام_الشهر تتطلب شهراً"))?;
            let y = args.get(1).and_then(|v| match v { Value::Integer(n) => Some(*n), _ => None }).unwrap_or(2024);
            let month = match m {
                Value::Integer(n) => *n,
                _ => return Err(RuntimeError::new_typed("استثناء_نوع", "ايام_الشهر تتطلب شهراً صحيحاً")),
            };
            let days_in_month = match month {
                1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
                4 | 6 | 9 | 11 => 30,
                2 => if (y % 4 == 0 && y % 100 != 0) || y % 400 == 0 { 29 } else { 28 },
                _ => return Err(RuntimeError::new_typed("استثناء_نوع", "الشهر يجب ان يكون 1-12")),
            };
            Ok(Value::Integer(days_in_month))
        }

        "عشوائي" => {
            use std::time::SystemTime;
            let t = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos() as u64;
            let val = (t >> 11) as f64 / (1u64 << 53) as f64;
            Ok(Value::Float(val))
        }
        "عشوائي_صحيح" => {
            if args.len() < 2 {
                return Err(RuntimeError::new("عشوائي_صحيح يتطلب معاملين (الحد الادنى، الحد الاقصى)"));
            }
            let min = match &args[0] {
                Value::Integer(n) => *n,
                _ => return Err(RuntimeError::new("الحد الادنى يجب ان يكون صحيحاً")),
            };
            let max = match &args[1] {
                Value::Integer(n) => *n,
                _ => return Err(RuntimeError::new("الحد الاقصى يجب ان يكون صحيحاً")),
            };
            use std::time::SystemTime;
            let t = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos() as u64;
            let range = (max - min + 1) as f64;
            let val = min + (((t >> 11) as f64 / (1u64 << 53) as f64) * range) as i64;
            Ok(Value::Integer(val))
        }

        "بذرة" => {
            if args.is_empty() {
                return Err(RuntimeError::new("البذرة تتطلب عدداً"));
            }
            let s = match &args[0] {
                Value::Integer(n) => *n as u64,
                Value::Float(f) => *f as u64,
                _ => return Err(RuntimeError::new("البذرة يجب ان يكون عدداً")),
            };
            // Store seed in thread-local for next calls
            use std::cell::Cell;
            thread_local! {
                static RNG_STATE: Cell<u64> = const { Cell::new(0) };
            }
            RNG_STATE.with(|state| state.set(s));
            Ok(Value::Null)
        }

        "منتظم" => {
            use std::cell::Cell;
            thread_local! {
                static RNG_STATE: Cell<u64> = const { Cell::new(0) };
            }
            let (a, b) = if args.len() >= 2 {
                let a = match &args[0] {
                    Value::Integer(n) => *n as f64,
                    Value::Float(f) => *f,
                    _ => return Err(RuntimeError::new("المعاملات يجب ان تكون اعداداً")),
                };
                let b = match &args[1] {
                    Value::Integer(n) => *n as f64,
                    Value::Float(f) => *f,
                    _ => return Err(RuntimeError::new("المعاملات يجب ان تكون اعداداً")),
                };
                (a, b)
            } else {
                (0.0, 1.0)
            };
            let seed = RNG_STATE.with(|s| {
                let mut val = s.get();
                if val == 0 {
                    val = std::time::SystemTime::now()
                        .duration_since(std::time::SystemTime::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_nanos() as u64;
                }
                val = val.wrapping_mul(6364136223846793005).wrapping_add(1);
                s.set(val);
                val
            });
            let ratio = (seed >> 11) as f64 / (1u64 << 53) as f64;
            Ok(Value::Float(a + ratio * (b - a)))
        }

        "استبدل" => {
            if args.len() < 3 {
                return Err(RuntimeError::new("استبدل يتطلب 3 معاملات"));
            }
            let s = match &args[0] {
                Value::String(s) => s.clone(),
                _ => return Err(RuntimeError::new("المعامل الاول يجب ان يكون نصاً")),
            };
            let from = match &args[1] {
                Value::String(s) => s,
                _ => return Err(RuntimeError::new("المعامل الثاني يجب ان يكون نصاً")),
            };
            let to = match &args[2] {
                Value::String(s) => s,
                _ => return Err(RuntimeError::new("المعامل الثالث يجب ان يكون نصاً")),
            };
            Ok(Value::String(Rc::new(s.replace(&**from, to))))
        }
        "اقسم" => {
            if args.is_empty() {
                return Err(RuntimeError::new("اقسم يتطلب نصاً على الاقل"));
            }
            let s = match &args[0] {
                Value::String(s) => s,
                _ => return Err(RuntimeError::new("المعامل الاول يجب ان يكون نصاً")),
            };
            let delimiter = if args.len() > 1 {
                match &args[1] {
                    Value::String(s) => s.clone(),
                    _ => return Err(RuntimeError::new("الفاصل يجب ان يكون نصاً")),
                }
            } else {
                Rc::new(" ".to_string())
            };
            let parts: Vec<Value> = s.split(&*delimiter)
                .map(|p| Value::String(Rc::new(p.to_string())))
                .collect();
            Ok(Value::List(SharedList::new(parts)))
        }
        "يحتوي" => {
            if args.len() < 2 {
                return Err(RuntimeError::new("يحتوي يتطلب معاملين"));
            }
            let s = match &args[0] {
                Value::String(s) => s,
                _ => return Err(RuntimeError::new("المعامل الاول يجب ان يكون نصاً")),
            };
            let pattern = match &args[1] {
                Value::String(s) => s,
                _ => return Err(RuntimeError::new("النمط يجب ان يكون نصاً")),
            };
            Ok(Value::Boolean(s.contains(&**pattern)))
        }
        "شطب" => {
            let s = match args.first() {
                Some(Value::String(s)) => s,
                _ => return Err(RuntimeError::new("المعامل يجب ان يكون نصاً")),
            };
            Ok(Value::String(s.trim().to_string().into()))
        }

        "استثناء" => {
            let message = args.first().map(|a| a.to_string_value()).unwrap_or_default();
            let class_name = if args.len() > 1 {
                args[1].to_string_value()
            } else {
                "استثناء".to_string()
            };
            Ok(Value::Exception(Box::new(ExceptionData { class_name, message, line: None, call_stack: Vec::new() })))
        }
        "ابعث" => {
            // send(generator, value) — resumes generator with a sent value
            let gen_val = args.first().ok_or_else(|| RuntimeError::new("ابعث يتطلب مولّد وقيمة"))?;
            let send_val = args.get(1).cloned().unwrap_or(Value::Null);
            match gen_val {
                Value::Generator(d) => {
                    vm.send_generator(d, send_val, module)
                }
                _ => Err(RuntimeError::new("المعامل الاول يجب ان يكون مولّداً")),
            }
        }

        "خطا_ن" => {
            let (class_name, message) = match args.len() {
                0 => return Err(RuntimeError::new("خطا_ن تتطلب رسالة على الاقل")),
                1 => ("استثناء_خطا".to_string(), args[0].to_string_value()),
                _ => (args[0].to_string_value(), args[1].to_string_value()),
            };
            if !vm.exception_hierarchy.contains_key(&class_name) {
                return Err(RuntimeError::new(format!("'{}' ليس صنف استثناء معروف", class_name)));
            }
            Ok(Value::Exception(Box::new(ExceptionData { class_name, message, line: None, call_stack: Vec::new() })))
        }

        "افتح" => {
            let path = args.first()
                .ok_or_else(|| RuntimeError::new("افتح تتطلب مسار ملف"))?
                .to_string_value();
            let mode = args.get(1)
                .map(|m| m.to_string_value())
                .unwrap_or_else(|| "ق".to_string());
            let file = match mode.as_str() {
                "ج" => std::fs::OpenOptions::new().read(true).write(true).create(true).truncate(false).open(&path),
                "ك" => std::fs::OpenOptions::new().write(true).create(true).truncate(true).open(&path),
                "ض" => std::fs::OpenOptions::new().append(true).create(true).open(&path),
                _ => std::fs::File::open(&path),
            };
            match file {
                Ok(f) => Ok(Value::File(crate::frame::FileHandle::new(f))),
                Err(e) => Err(RuntimeError::new(format!("خطا في فتح الملف '{}': {}", path, e))),
            }
        }

        "تتبع" => {
            let iterable = args.first().ok_or_else(|| RuntimeError::new("تتبع تتطلب عنصر واحد على الاقل"))?;
            let start = get_optional_int_arg(args, 1).unwrap_or(0);
            let items: Vec<Value> = match iterable {
                Value::List(l) => l.borrow().clone(),
                Value::Tuple(t) => t.as_ref().clone(),
                Value::String(s) => s.chars().map(|c| Value::String(Rc::new(c.to_string()))).collect(),
                Value::Range(d) => {
                    let mut items = Vec::new();
                    let mut cur = d.start;
                    if d.step > 0 {
                        while cur < d.end { items.push(Value::Integer(cur)); cur += d.step; }
                    } else if d.step < 0 {
                        while cur > d.end { items.push(Value::Integer(cur)); cur += d.step; }
                    }
                    items
                }
                _ => return Err(RuntimeError::new_typed("استثناء_نوع", "تتبع يتطلب قائمة او مترابطة او نص او نطاق")),
            };
            let result: Vec<Value> = items.into_iter().enumerate()
                .map(|(i, v)| Value::Tuple(Rc::new(vec![Value::Integer(start + i as i64), v])))
                .collect();
            Ok(Value::List(SharedList::new(result)))
        }

        "ضغط" => {
            if args.len() < 2 {
                return Err(RuntimeError::new("ضغط يتطلب دالة وقائمة"));
            }
            let func = &args[0];
            let iterable = &args[1];
            let items: Vec<Value> = match iterable {
                Value::List(l) => l.borrow().clone(),
                Value::Tuple(t) => t.as_ref().clone(),
                Value::String(s) => s.chars().map(|c| Value::String(Rc::new(c.to_string()))).collect(),
                _ => return Err(RuntimeError::new_typed("استثناء_نوع", "ضغط يتطلب قائمة او مترابطة او نص")),
            };
            let mut result = Vec::new();
            for item in items {
                let mapped = func.call(&[item], &[], vm, module)?;
                result.push(mapped);
            }
            Ok(Value::List(SharedList::new(result)))
        }

        "تصفية" => {
            if args.len() < 2 {
                return Err(RuntimeError::new("تصفية يتطلب دالة وقائمة"));
            }
            let func = &args[0];
            let iterable = &args[1];
            let items: Vec<Value> = match iterable {
                Value::List(l) => l.borrow().clone(),
                Value::Tuple(t) => t.as_ref().clone(),
                Value::String(s) => s.chars().map(|c| Value::String(Rc::new(c.to_string()))).collect(),
                _ => return Err(RuntimeError::new_typed("استثناء_نوع", "تصفية تتطلب قائمة او مترابطة او نص")),
            };
            let mut result = Vec::new();
            for item in items {
                let keep = func.call(std::slice::from_ref(&item), &[], vm, module)?;
                if keep.is_truthy() {
                    result.push(item);
                }
            }
            Ok(Value::List(SharedList::new(result)))
        }

        "عد" => {
            if args.is_empty() {
                return Err(RuntimeError::new("عد يتطلب قائمة على الاقل"));
            }
            let iterable = &args[0];
            let start: i64 = if let Some(v) = kwargs.iter().find(|(k, _)| k == "البداية") {
                match &v.1 {
                    Value::Integer(n) => *n,
                    _ =>                     return Err(RuntimeError::new("البداية يجب ان يكون عدداً صحيحاً")),
                }
            } else {
                match args.get(1) {
                    Some(Value::Integer(n)) => *n,
                    _ => 0,
                }
            };
            let items: Vec<Value> = match iterable {
                Value::List(l) => l.borrow().clone(),
                Value::Tuple(t) => t.as_ref().clone(),
                Value::String(s) => s.chars().map(|c| Value::String(Rc::new(c.to_string()))).collect(),
                _ => return Err(RuntimeError::new_typed("استثناء_نوع", "عد يتطلب قائمة او مترابطة او نص")),
            };
            let mut result = Vec::with_capacity(items.len());
            let mut idx = start;
            for item in items {
                result.push(Value::Tuple(Rc::new(vec![Value::Integer(idx), item])));
                idx += 1;
            }
            Ok(Value::List(SharedList::new(result)))
        }

        "اقتران" => {
            if args.len() < 2 {
                return Err(RuntimeError::new("اقتران يتطلب قائمتين على الاقل"));
            }
            let mut iterables: Vec<Vec<Value>> = Vec::with_capacity(args.len());
            for arg in args {
                let items: Vec<Value> = match arg {
                    Value::List(l) => l.borrow().clone(),
                    Value::Tuple(t) => t.as_ref().clone(),
                    Value::String(s) => s.chars().map(|c| Value::String(Rc::new(c.to_string()))).collect(),
                    _ => return Err(RuntimeError::new_typed("استثناء_نوع", "اقتران يتطلب قوائم او مرابطة او نص")),
                };
                iterables.push(items);
            }
            let min_len = iterables.iter().map(|v| v.len()).min().unwrap_or(0);
            let mut result = Vec::with_capacity(min_len);
            for i in 0..min_len {
                let tuple_vals: Vec<Value> = iterables.iter().map(|v| v[i].clone()).collect();
                result.push(Value::Tuple(Rc::new(tuple_vals)));
            }
            Ok(Value::List(SharedList::new(result)))
        }

        "اختزل" | "اختزال" => {
            if args.len() < 2 {
                return Err(RuntimeError::new("اختزل يتطلب دالة وقائمة"));
            }
            let func = &args[0];
            let iterable = &args[1];
            let items: Vec<Value> = match iterable {
                Value::List(l) => l.borrow().clone(),
                Value::Tuple(t) => t.as_ref().clone(),
                Value::String(s) => s.chars().map(|c| Value::String(Rc::new(c.to_string()))).collect(),
                _ => return Err(RuntimeError::new_typed("استثناء_نوع", "اختزل يتطلب قائمة او مترابطة او نص")),
            };
            let mut acc = if args.len() > 2 {
                args[2].clone()
            } else if items.is_empty() {
                return Err(RuntimeError::new("اختزل تتطلب بذرة اذا كانت القائمة فارغة"));
            } else {
                items[0].clone()
            };
            let start = if args.len() > 2 { 0 } else { 1 };
            for item in &items[start..] {
                acc = func.call(&[acc, item.clone()], &[], vm, module)?;
            }
            Ok(acc)
        }

        "كائن_تحليل" => {
            let s = args.first()
                .ok_or_else(|| RuntimeError::new("كائن.تحليل يتطلب نصاً"))?
                .to_string_value();
            let json_val: serde_json::Value = serde_json::from_str(&s)
                .map_err(|e| RuntimeError::new(format!("خطا في تحليل JSON: {}", e)))?;
            Ok(json_value_to_arabi(&json_val))
        }

        "كائن_تحويل" => {
            let val = args.first()
                .ok_or_else(|| RuntimeError::new("كائن.تحويل يتطلب قيمة"))?;
            let json_val = arabi_value_to_json(val)?;
            let s = serde_json::to_string(&json_val)
                .map_err(|e| RuntimeError::new(format!("خطا في تحويل JSON: {}", e)))?;
            Ok(Value::String(s.into()))
        }

        "كائن_تحليل_ملف" => {
            let path = args.first()
                .ok_or_else(|| RuntimeError::new("كائن.تحليل_ملف يتطلب مسار ملف"))?
                .to_string_value();
            let content = std::fs::read_to_string(&path)
                .map_err(|e| RuntimeError::new(format!("خطا في قراءة الملف '{}': {}", path, e)))?;
            let json_val: serde_json::Value = serde_json::from_str(&content)
                .map_err(|e| RuntimeError::new(format!("خطا في تحليل JSON: {}", e)))?;
            Ok(json_value_to_arabi(&json_val))
        }

        "كائن_اخرج_ملف" => {
            let path = args.first()
                .ok_or_else(|| RuntimeError::new("كائن.اخرج_ملف يتطلب مسار ملف"))?
                .to_string_value();
            let val = args.get(1)
                .ok_or_else(|| RuntimeError::new("كائن.اخرج_ملف يتطلب قيمة"))?;
            let json_val = arabi_value_to_json(val)?;
            let s = serde_json::to_string_pretty(&json_val)
                .map_err(|e| RuntimeError::new(format!("خطا في تحويل JSON: {}", e)))?;
            std::fs::write(&path, s)
                .map_err(|e| RuntimeError::new(format!("خطا في كتابة الملف '{}': {}", path, e)))?;
            Ok(Value::Null)
        }

        "نظام_ادارة" => {
            match std::env::current_dir() {
                Ok(path) => Ok(Value::String(path.to_string_lossy().to_string().into())),
                Err(e) => Err(RuntimeError::new(format!("خطا في الحصول على الدirectory: {}", e))),
            }
        }
        "نظام_ادخل_ادارة" => {
            let path = args.first()
                .ok_or_else(|| RuntimeError::new("نظام.ادخل_ادارة يتطلب مساراً"))?
                .to_string_value();
            std::env::set_current_dir(&path)
                .map_err(|e| RuntimeError::new(format!("خطا في تغيير الدirectory '{}': {}", path, e)))?;
            Ok(Value::Null)
        }
        "نظام_قائمة_ادارة" => {
            let path = args.first()
                .map(|a| a.to_string_value())
                .unwrap_or_else(|| ".".to_string());
            let entries: Vec<Value> = std::fs::read_dir(&path)
                .map_err(|e| RuntimeError::new(format!("خطا في قراءة الدirectory '{}': {}", path, e)))?
                .filter_map(|e| e.ok())
                .map(|e| Value::String(e.file_name().to_string_lossy().to_string().into()))
                .collect();
            Ok(Value::List(SharedList::new(entries)))
        }
        "نظام_اداء" => {
            let path = args.first()
                .ok_or_else(|| RuntimeError::new("نظام.اداء يتطلب مساراً"))?
                .to_string_value();
            let meta = std::fs::metadata(&path)
                .map_err(|e| RuntimeError::new(format!("خطا في الاطلاع على '{}': {}", path, e)))?;
            let mut map = Vec::new();
            map.push((Value::String(Rc::new("النوع".to_string())), Value::String(Rc::new(if meta.is_dir() { "ادارة".to_string() } else { "ملف".to_string() }))));
            map.push((Value::String(Rc::new("الحجم".to_string())), Value::Integer(meta.len() as i64)));
            map.push((Value::String(Rc::new("قابل_للكتابة".to_string())), Value::Boolean(!meta.permissions().readonly())));
            if let Ok(modified) = meta.modified() {
                let elapsed = modified.duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
                map.push((Value::String(Rc::new("تعديل".to_string())), Value::Integer(elapsed.as_secs() as i64)));
            }
            Ok(Value::Dict(SharedDict::new(map)))
        }
        "نظام_وجود" => {
            let path = args.first()
                .ok_or_else(|| RuntimeError::new("نظام.وجود يتطلب مساراً"))?
                .to_string_value();
            Ok(Value::Boolean(std::path::Path::new(&path).exists()))
        }
        "نظام_انشئ_ادارة" => {
            let path = args.first()
                .ok_or_else(|| RuntimeError::new("نظام.انشئ_ادارة يتطلب مساراً"))?
                .to_string_value();
            std::fs::create_dir_all(&path)
                .map_err(|e| RuntimeError::new(format!("خطا في انشاء الدirectory '{}': {}", path, e)))?;
            Ok(Value::Null)
        }
        "نظام_احذف_ملف" => {
            let path = args.first()
                .ok_or_else(|| RuntimeError::new("نظام.احذف_ملف يتطلب مساراً"))?
                .to_string_value();
            let p = std::path::Path::new(&path);
            if p.is_dir() {
                std::fs::remove_dir_all(&path)
                    .map_err(|e| RuntimeError::new(format!("خطا في حذف الدirectory '{}': {}", path, e)))?;
            } else {
                std::fs::remove_file(&path)
                    .map_err(|e| RuntimeError::new(format!("خطا في حذف الملف '{}': {}", path, e)))?;
            }
            Ok(Value::Null)
        }
        "نظام_اسم" => {
            Ok(Value::String(Rc::new(std::env::consts::OS.to_string())))
        }
        "نظام_ادخال" => {
            let prompt = args.first().map(|a| a.to_string_value()).unwrap_or_default();
            use std::io::Write;
            if !prompt.is_empty() {
                print!("{}", prompt);
                std::io::stdout().flush().ok();
            }
            let mut input = String::new();
            std::io::stdin().read_line(&mut input)
                .map_err(|e| RuntimeError::new(format!("خطا في القراءة: {}", e)))?;
            Ok(Value::String(input.trim_end_matches('\n').trim_end_matches('\r').to_string().into()))
        }
        "نظام_احصل_على" => {
            let name = args.first()
                .ok_or_else(|| RuntimeError::new("نظام.احصل_على يتطلب اسم المتغير"))?
                .to_string_value();
            match std::env::var(&name) {
                Ok(val) => Ok(Value::String(val.into())),
                Err(_) => Ok(Value::Null),
            }
        }
        "نظام_ادخل_بيئة" => {
            let name = args.first()
                .ok_or_else(|| RuntimeError::new("نظام.ادخل_بيئة يتطلب اسماً"))?
                .to_string_value();
            let value = args.get(1)
                .ok_or_else(|| RuntimeError::new("نظام.ادخل_بيئة يتطلب قيمة"))?
                .to_string_value();
            std::env::set_var(&name, &value);
            Ok(Value::Null)
        }
        "نظام_اسم_ملف" => {
            let path = args.first()
                .ok_or_else(|| RuntimeError::new("نظام.اسم_ملف يتطلب مساراً"))?
                .to_string_value();
            let p = std::path::Path::new(&path);
            let name = p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
            Ok(Value::String(name.into()))
        }
        "نظام_ادارة_ملف" => {
            let path = args.first()
                .ok_or_else(|| RuntimeError::new("نظام.ادارة_ملف يتطلب مساراً"))?
                .to_string_value();
            let p = std::path::Path::new(&path);
            let dir = p.parent().map(|d| d.to_string_lossy().to_string()).unwrap_or_default();
            Ok(Value::String(dir.into()))
        }
        "نظام_امتداد" => {
            let path = args.first()
                .ok_or_else(|| RuntimeError::new("نظام.امتداد يتطلب مساراً"))?
                .to_string_value();
            let p = std::path::Path::new(&path);
            let ext = p.extension().map(|e| e.to_string_lossy().to_string()).unwrap_or_default();
            Ok(Value::String(ext.into()))
        }

        "مرتب" => {
            let obj = args.first().ok_or_else(|| RuntimeError::new("مرتب يتطلب معامل واحد"))?;
            let key_func = if args.len() > 1 { Some(&args[1]) } else { None };
            match obj {
                Value::List(items) => {
                    let mut sorted_items: Vec<Value> = items.borrow().clone();
                    if let Some(key) = key_func {
                        sorted_items.sort_by(|a, b| {
                            let ka = key.call(&[a.clone()], &[], vm, module).unwrap_or(Value::Null);
                            let kb = key.call(&[b.clone()], &[], vm, module).unwrap_or(Value::Null);
                            ka.partial_cmp(&kb).unwrap_or(std::cmp::Ordering::Equal)
                        });
                    } else {
                        sorted_items.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                    }
                    Ok(Value::List(SharedList::new(sorted_items)))
                }
                Value::Tuple(items) => {
                    let mut sorted_items: Vec<Value> = items.as_ref().clone();
                    if let Some(key) = key_func {
                        sorted_items.sort_by(|a, b| {
                            let ka = key.call(&[a.clone()], &[], vm, module).unwrap_or(Value::Null);
                            let kb = key.call(&[b.clone()], &[], vm, module).unwrap_or(Value::Null);
                            ka.partial_cmp(&kb).unwrap_or(std::cmp::Ordering::Equal)
                        });
                    } else {
                        sorted_items.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                    }
                    Ok(Value::List(SharedList::new(sorted_items)))
                }
                Value::String(s) => {
                    let mut chars: Vec<String> = s.chars().map(|c| c.to_string()).collect();
                    chars.sort();
                    let result: String = chars.join("");
                    Ok(Value::String(result.into()))
                }
                _ => Err(RuntimeError::new_typed("استثناء_نوع", "مرتب يتطلب قائمة او مترابطة او نص")),
            }
        }

        "حرف" => {
            let obj = args.first().ok_or_else(|| RuntimeError::new("حرف يتطلب معامل واحد"))?;
            match obj {
                Value::Integer(n) => {
                    let c = char::from_u32(*n as u32)
                        .ok_or_else(|| RuntimeError::new_typed("استثناء_نوع", "رقم غير صالح لحرف"))?;
                    Ok(Value::String(Rc::new(c.to_string())))
                }
                _ => Err(RuntimeError::new_typed("استثناء_نوع", "حرف يتطلب رقماً")),
            }
        }

        "رقم" => {
            let obj = args.first().ok_or_else(|| RuntimeError::new("رقم يتطلب معامل واحد"))?;
            match obj {
                Value::String(s) => {
                    let c = s.chars().next().ok_or_else(|| RuntimeError::new_typed("استثناء_نوع", "نص فارغ"))?;
                    Ok(Value::Integer(c as i64))
                }
                _ => Err(RuntimeError::new_typed("استثناء_نوع", "رقم يتطلب نصاً")),
            }
        }

        "ست_عشري" => {
            let obj = args.first().ok_or_else(|| RuntimeError::new("ست_عشري يتطلب معامل واحد"))?;
            match obj {
                Value::Integer(n) => Ok(Value::String(Rc::new(format!("0x{:x}", n)))),
                _ => Err(RuntimeError::new_typed("استثناء_نوع", "ست_عشري يتطلب رقماً")),
            }
        }

        // === Math Extended ===
        "اكبر_قاسم" => {
            let a = args.get(0).ok_or_else(|| RuntimeError::new("gcd يتطلب معاملين"))?;
            let b = args.get(1).ok_or_else(|| RuntimeError::new("gcd يتطلب معاملين"))?;
            let (mut x, mut y) = match (a, b) {
                (Value::Integer(x), Value::Integer(y)) => (*x, *y),
                _ => return Err(RuntimeError::new_typed("استثناء_نوع", "gcd يتطلب عددين صحيحين")),
            };
            while y != 0 { let t = y; y = x % y; x = t; }
            Ok(Value::Integer(x.abs()))
        }
        "اصغر_مضاعف" => {
            let a = args.get(0).ok_or_else(|| RuntimeError::new("lcm يتطلب معاملين"))?;
            let b = args.get(1).ok_or_else(|| RuntimeError::new("lcm يتطلب معاملين"))?;
            match (a, b) {
                (Value::Integer(x), Value::Integer(y)) => {
                    if *x == 0 || *y == 0 { return Ok(Value::Integer(0)); }
                    let gcd_val = {
                        let (mut a2, mut b2) = (x.abs(), y.abs());
                        while b2 != 0 { let t = b2; b2 = a2 % b2; a2 = t; }
                        a2
                    };
                    Ok(Value::Integer((x.abs() / gcd_val) * y.abs()))
                }
                _ => Err(RuntimeError::new_typed("استثناء_نوع", "lcm يتطلب عددين صحيحين")),
            }
        }
        "هل_اولية" => {
            let n = args.first().ok_or_else(|| RuntimeError::new("هل_اولية يتطلب رقماً"))?;
            match n {
                Value::Integer(n) => {
                    if *n < 2 { return Ok(Value::Boolean(false)); }
                    if *n == 2 { return Ok(Value::Boolean(true)); }
                    if *n % 2 == 0 { return Ok(Value::Boolean(false)); }
                    let mut i = 3;
                    while i * i <= *n {
                        if *n % i == 0 { return Ok(Value::Boolean(false)); }
                        i += 2;
                    }
                    Ok(Value::Boolean(true))
                }
                _ => Err(RuntimeError::new_typed("استثناء_نوع", "هل_اولية يتطلب رقماً")),
            }
        }
        "اولية" => {
            let n = args.first().ok_or_else(|| RuntimeError::new("اولية تتطلب رقماً"))?;
            match n {
                Value::Integer(n) => {
                    if *n < 2 { return Ok(Value::List(SharedList::new(vec![]))); }
                    let mut sieve = vec![true; (*n + 1) as usize];
                    sieve[0] = false;
                    if *n >= 1 { sieve[1] = false; }
                    let mut i = 2;
                    while i * i <= *n as usize {
                        if sieve[i] {
                            let mut j = i * i;
                            while j <= *n as usize {
                                sieve[j] = false;
                                j += i;
                            }
                        }
                        i += 1;
                    }
                    let primes: Vec<Value> = sieve.iter().enumerate()
                        .filter(|(_, &is_p)| is_p)
                        .map(|(i, _)| Value::Integer(i as i64))
                        .collect();
                    Ok(Value::List(SharedList::new(primes)))
                }
                _ => Err(RuntimeError::new_typed("استثناء_نوع", "اولية تتطلب رقماً")),
            }
        }
        "فيبوناتشي" => {
            let n = args.first().ok_or_else(|| RuntimeError::new("فيبوناتشي تتطلب رقماً"))?;
            match n {
                Value::Integer(n) => {
                    if *n <= 0 { return Ok(Value::Integer(0)); }
                    if *n == 1 { return Ok(Value::Integer(1)); }
                    let mut a = 0i64;
                    let mut b = 1i64;
                    for _ in 2..=*n {
                        let tmp = a + b;
                        a = b;
                        b = tmp;
                    }
                    Ok(Value::Integer(b))
                }
                _ => Err(RuntimeError::new_typed("استثناء_نوع", "فيبوناتشي تتطلب رقماً")),
            }
        }
        "ترتيبي" => {
            let n = args.get(0).ok_or_else(|| RuntimeError::new("ترتيبي يتطلب معاملين"))?;
            let r = args.get(1).ok_or_else(|| RuntimeError::new("ترتيبي يتطلب معاملين"))?;
            match (n, r) {
                (Value::Integer(n), Value::Integer(r)) => {
                    if *r > *n || *r < 0 { return Ok(Value::Integer(0)); }
                    let mut result = 1i64;
                    for i in 0..*r { result *= *n - i; }
                    Ok(Value::Integer(result))
                }
                _ => Err(RuntimeError::new_typed("استثناء_نوع", "ترتيبي يتطلب عددين صحيحين")),
            }
        }
        "تركيبي" => {
            let n = args.get(0).ok_or_else(|| RuntimeError::new("تركيبي يتطلب معاملين"))?;
            let r = args.get(1).ok_or_else(|| RuntimeError::new("تركيبي يتطلب معاملين"))?;
            match (n, r) {
                (Value::Integer(n), Value::Integer(r)) => {
                    if *r > *n || *r < 0 { return Ok(Value::Integer(0)); }
                    let r = if *r > *n - *r { *n - *r } else { *r };
                    let mut result = 1i64;
                    for i in 0..r { result = result * (*n - i) / (i + 1); }
                    Ok(Value::Integer(result))
                }
                _ => Err(RuntimeError::new_typed("استثناء_نوع", "تركيبي يتطلب عددين صحيحين")),
            }
        }
        "انحراف_معياري" => {
            let list_arg = args.first().ok_or_else(|| RuntimeError::new("انحراف_معياري تتطلب مصفوفة"))?;
            let items = match list_arg {
                Value::List(list) => list.borrow().clone(),
                _ => return Err(RuntimeError::new_typed("استثناء_نوع", "انحراف_معياري تتطلب مصفوفة")),
            };
            if items.is_empty() { return Ok(Value::Float(0.0)); }
            let sum: f64 = items.iter().map(|v| match v { Value::Integer(i) => *i as f64, Value::Float(f) => *f, _ => 0.0 }).sum();
            let mean = sum / items.len() as f64;
            let variance: f64 = items.iter().map(|v| {
                let val = match v { Value::Integer(i) => *i as f64, Value::Float(f) => *f, _ => 0.0 };
                (val - mean).powi(2)
            }).sum::<f64>() / items.len() as f64;
            Ok(Value::Float(variance.sqrt()))
        }
        "وسيط" => {
            let list_arg = args.first().ok_or_else(|| RuntimeError::new("وسيط تتطلب مصفوفة"))?;
            let mut items: Vec<f64> = match list_arg {
                Value::List(list) => list.borrow().iter().map(|v| match v { Value::Integer(i) => *i as f64, Value::Float(f) => *f, _ => 0.0 }).collect(),
                _ => return Err(RuntimeError::new_typed("استثناء_نوع", "وسيط تتطلب مصفوفة")),
            };
            if items.is_empty() { return Ok(Value::Float(0.0)); }
            items.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            let n = items.len();
            if n % 2 == 1 { Ok(Value::Float(items[n / 2])) }
            else { Ok(Value::Float((items[n / 2 - 1] + items[n / 2]) / 2.0)) }
        }

        // === String Extended ===
        "بداية_بـ" => {
            let s = args.get(0).ok_or_else(|| RuntimeError::new("بداية_بـ يتطلب نصاً وبحثاً"))?;
            let prefix = args.get(1).ok_or_else(|| RuntimeError::new("بداية_بـ يتطلب نصاً وبحثاً"))?;
            match (s, prefix) {
                (Value::String(s), Value::String(p)) => Ok(Value::Boolean(s.starts_with(p.as_ref()))),
                _ => Err(RuntimeError::new_typed("استثناء_نوع", "بداية_بـ يتطلب نصين")),
            }
        }
        "نهاية_بـ" => {
            let s = args.get(0).ok_or_else(|| RuntimeError::new("نهاية_بـ يتطلب نصاً وبحثاً"))?;
            let suffix = args.get(1).ok_or_else(|| RuntimeError::new("نهاية_بـ يتطلب نصاً وبحثاً"))?;
            match (s, suffix) {
                (Value::String(s), Value::String(p)) => Ok(Value::Boolean(s.ends_with(p.as_ref()))),
                _ => Err(RuntimeError::new_typed("استثناء_نوع", "نهاية_بـ يتطلب نصين")),
            }
        }
        "اعلى" => {
            let s = args.first().ok_or_else(|| RuntimeError::new("اعلى يتطلب نصاً"))?;
            match s {
                Value::String(s) => Ok(Value::String(Rc::new(s.to_uppercase()))),
                _ => Err(RuntimeError::new_typed("استثناء_نوع", "اعلى يتطلب نصاً")),
            }
        }
        "اسفل" => {
            let s = args.first().ok_or_else(|| RuntimeError::new("اسفل يتطلب نصاً"))?;
            match s {
                Value::String(s) => Ok(Value::String(Rc::new(s.to_lowercase()))),
                _ => Err(RuntimeError::new_typed("استثناء_نوع", "اسفل يتطلب نصاً")),
            }
        }
        "تكرار_اعلى" => {
            let s = args.first().ok_or_else(|| RuntimeError::new("تكرار_اعلى يتطلب نصاً"))?;
            match s {
                Value::String(s) => {
                    let titled: String = s.split(' ').map(|word| {
                        let mut chars = word.chars();
                        match chars.next() {
                            None => String::new(),
                            Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
                        }
                    }).collect::<Vec<_>>().join(" ");
                    Ok(Value::String(Rc::new(titled)))
                }
                _ => Err(RuntimeError::new_typed("استثناء_نوع", "تكرار_اعلى يتطلب نصاً")),
            }
        }
        "ملء" => {
            let s = args.get(0).ok_or_else(|| RuntimeError::new("ملء يتطلب نصاً وعرضماً وحراً"))?;
            let width = match args.get(1) { Some(Value::Integer(n)) => *n as usize, _ => return Err(RuntimeError::new("ملء يتطلب عرضاً")) };
            let ch = match args.get(2) { Some(Value::String(c)) => c.chars().next().unwrap_or(' '), _ => ' ' };
            match s {
                Value::String(s) => {
                    if s.len() >= width { return Ok(Value::String(s.clone())); }
                    let pad_len = width - s.len();
                    let pad_str: String = std::iter::repeat(ch).take(pad_len).collect();
                    Ok(Value::String(Rc::new(format!("{}{}", pad_str, s.as_ref()))))
                }
                _ => Err(RuntimeError::new_typed("استثناء_نوع", "ملء يتطلب نصاً")),
            }
        }
        "توسيط" => {
            let s = args.get(0).ok_or_else(|| RuntimeError::new("توسيط يتطلب نصاً وعرضماً"))?;
            let width = match args.get(1) { Some(Value::Integer(n)) => *n as usize, _ => return Err(RuntimeError::new("توسيط يتطلب عرضاً")) };
            let ch = match args.get(2) { Some(Value::String(c)) => c.chars().next().unwrap_or(' '), _ => ' ' };
            match s {
                Value::String(s) => {
                    if s.len() >= width { return Ok(Value::String(s.clone())); }
                    let total_pad = width - s.len();
                    let left = total_pad / 2;
                    let right = total_pad - left;
                    let left_pad: String = std::iter::repeat(ch).take(left).collect();
                    let right_pad: String = std::iter::repeat(ch).take(right).collect();
                    Ok(Value::String(Rc::new(format!("{}{}{}", left_pad, s.as_ref(), right_pad))))
                }
                _ => Err(RuntimeError::new_typed("استثناء_نوع", "توسيط يتطلب نصاً")),
            }
        }
        "قطع" => {
            let s = args.get(0).ok_or_else(|| RuntimeError::new("قطع يتطلب نصاً وبدايةً ونهايةً"))?;
            let start = match args.get(1) { Some(Value::Integer(n)) => *n as usize, _ => 0 };
            let end = args.get(2).and_then(|v| match v { Value::Integer(n) => Some(*n as usize), _ => None });
            match s {
                Value::String(s) => {
                    let chars: Vec<char> = s.chars().collect();
                    let end_idx = end.unwrap_or(chars.len()).min(chars.len());
                    let start_idx = start.min(chars.len());
                    let result: String = chars[start_idx..end_idx].iter().collect();
                    Ok(Value::String(Rc::new(result)))
                }
                _ => Err(RuntimeError::new_typed("استثناء_نوع", "قطع يتطلب نصاً")),
            }
        }
        "عدد" => {
            if args.len() < 2 { return Err(RuntimeError::new("عدد يتطلب نصاً ونمطاً")); }
            let s = match &args[0] { Value::String(s) => s.as_str(), _ => return Err(RuntimeError::new("المعامل الاول يجب ان يكون نصاً")) };
            let pattern = match &args[1] { Value::String(p) => p.as_str(), _ => return Err(RuntimeError::new("النمط يجب ان يكون نصاً")) };
            Ok(Value::Integer(s.matches(pattern).count() as i64))
        }
        "اوجد" => {
            if args.len() < 2 { return Err(RuntimeError::new("اوجد يتطلب نصاً ونمطاً")); }
            let s = match &args[0] { Value::String(s) => s.as_str(), _ => return Err(RuntimeError::new("المعامل الاول يجب ان يكون نصاً")) };
            let pattern = match &args[1] { Value::String(p) => p.as_str(), _ => return Err(RuntimeError::new("النمط يجب ان يكون نصاً")) };
            let start = match args.get(2) { Some(Value::Integer(n)) => *n as usize, _ => 0 };
            match s[start..].find(pattern) {
                Some(pos) => Ok(Value::Integer((start + pos) as i64)),
                None => Ok(Value::Integer(-1)),
            }
        }
        "اوجد_النهاية" => {
            if args.len() < 2 { return Err(RuntimeError::new("اوجد_النهاية يتطلب نصاً ونمطاً")); }
            let s = match &args[0] { Value::String(s) => s.as_str(), _ => return Err(RuntimeError::new("المعامل الاول يجب ان يكون نصاً")) };
            let pattern = match &args[1] { Value::String(p) => p.as_str(), _ => return Err(RuntimeError::new("النمط يجب ان يكون نصاً")) };
            match s.rfind(pattern) {
                Some(pos) => Ok(Value::Integer(pos as i64)),
                None => Ok(Value::Integer(-1)),
            }
        }
        "حرف_البداية" => {
            if args.is_empty() { return Err(RuntimeError::new("حرف_البداية يتطلب نصاً")); }
            let s = match &args[0] { Value::String(s) => s.as_str(), _ => return Err(RuntimeError::new("المعامل يجب ان يكون نصاً")) };
            Ok(s.chars().next().map(|c| Value::Integer(c as i64)).unwrap_or(Value::Integer(-1)))
        }
        "حرف_النهاية" => {
            if args.is_empty() { return Err(RuntimeError::new("حرف_النهاية يتطلب نصاً")); }
            let s = match &args[0] { Value::String(s) => s.as_str(), _ => return Err(RuntimeError::new("المعامل يجب ان يكون نصاً")) };
            Ok(s.chars().last().map(|c| Value::Integer(c as i64)).unwrap_or(Value::Integer(-1)))
        }
        "معكوس_نص" => {
            if args.is_empty() { return Err(RuntimeError::new("معكوس_نص يتطلب نصاً")); }
            let s = match &args[0] { Value::String(s) => s.as_str(), _ => return Err(RuntimeError::new("المعامل يجب ان يكون نصاً")) };
            let reversed: String = s.chars().rev().collect();
            Ok(Value::String(Rc::new(reversed)))
        }
        "تكرار" => {
            if args.len() < 2 { return Err(RuntimeError::new("تكرار يتطلب نصاً وعدد مرات")); }
            let s = match &args[0] { Value::String(s) => s.as_str(), _ => return Err(RuntimeError::new("المعامل الاول يجب ان يكون نصاً")) };
            let n = match &args[1] { Value::Integer(n) => *n, _ => return Err(RuntimeError::new("عدد المرات يجب ان يكون صحيحاً")) };
            if n < 0 { return Err(RuntimeError::new("عدد المرات لا يمكن ان يكون سالباً")); }
            Ok(Value::String(Rc::new(s.repeat(n as usize))))
        }
        "اربط" => {
            if args.len() < 2 { return Err(RuntimeError::new("اربط يتطلب فاصلاً ومصفوفة")); }
            let sep = match &args[0] { Value::String(s) => s.as_str(), _ => return Err(RuntimeError::new("الفصل يجب ان يكون نصاً")) };
            let list = match &args[1] { Value::List(l) => l.borrow(), _ => return Err(RuntimeError::new("المعامل الثاني يجب ان يكون مصفوفة")) };
            let parts: Vec<String> = list.iter().map(|v| v.to_string_value()).collect();
            Ok(Value::String(Rc::new(parts.join(sep))))
        }
        "تحقق_من_الحرف" => {
            if args.len() < 2 { return Err(RuntimeError::new("تحقق_من_الحرف يتطلب نصاً ونوعاً")); }
            let s = match &args[0] { Value::String(s) => s.as_str(), _ => return Err(RuntimeError::new("المعامل الاول يجب ان يكون نصاً")) };
            let kind = match &args[1] { Value::String(k) => k.as_str(), _ => return Err(RuntimeError::new("النوع يجب ان يكون نصاً")) };
            if s.is_empty() { return Ok(Value::Boolean(false)); }
            let result = match kind {
                "حرف" | "isalpha" => s.chars().all(|c| c.is_alphabetic()),
                "رقم" | "isdigit" => s.chars().all(|c| c.is_ascii_digit()),
                "رقم_او_حرف" | "isalnum" => s.chars().all(|c| c.is_alphanumeric()),
                "مسافة" | "isspace" => s.chars().all(|c| c.is_whitespace()),
                "صغير" | "islower" => s.chars().filter(|c| c.is_alphabetic()).all(|c| c.is_lowercase()),
                "كبير" | "isupper" => s.chars().filter(|c| c.is_alphabetic()).all(|c| c.is_uppercase()),
                _ => return Err(RuntimeError::new(format!("نوع غير معروف: {}", kind))),
            };
            Ok(Value::Boolean(result))
        }
        "اقلب" => {
            if args.is_empty() { return Err(RuntimeError::new("اقلب يتطلب نصاً")); }
            let s = match &args[0] { Value::String(s) => s.as_str(), _ => return Err(RuntimeError::new("المعامل يجب ان يكون نصاً")) };
            let result: String = s.chars().map(|c| {
                if c.is_uppercase() { c.to_lowercase().to_string() } else { c.to_uppercase().to_string() }
            }).collect();
            Ok(Value::String(Rc::new(result)))
        }
        "ملء_صفر" => {
            if args.is_empty() { return Err(RuntimeError::new("ملء_صفر يتطلب نصاً")); }
            let s = match &args[0] { Value::String(s) => s.as_str(), _ => return Err(RuntimeError::new("المعامل يجب ان يكون نصاً")) };
            let width = match args.get(1) { Some(Value::Integer(n)) => *n as usize, _ => return Err(RuntimeError::new("العرض مطلوب")) };
            let pad_len = width.saturating_sub(s.len());
            let padded = format!("{}{}", "0".repeat(pad_len), s);
            Ok(Value::String(Rc::new(padded)))
        }
        "اول_حرف_كبير" => {
            if args.is_empty() { return Err(RuntimeError::new("اول_حرف_كبير يتطلب نصاً")); }
            let s = match &args[0] { Value::String(s) => s.as_str(), _ => return Err(RuntimeError::new("المعامل يجب ان يكون نصاً")) };
            let mut chars = s.chars();
            match chars.next() {
                Some(first) => {
                    let rest: String = chars.collect();
                    Ok(Value::String(Rc::new(format!("{}{}", first.to_uppercase(), rest.to_lowercase()))))
                }
                None => Ok(Value::String(Rc::new(String::new()))),
            }
        }
        "كل_اول_حرف_كبير" => {
            if args.is_empty() { return Err(RuntimeError::new("كل_اول_حرف_كبير يتطلب نصاً")); }
            let s = match &args[0] { Value::String(s) => s.as_str(), _ => return Err(RuntimeError::new("المعامل يجب ان يكون نصاً")) };
            let result: String = s.split_whitespace().map(|word| {
                let mut chars = word.chars();
                match chars.next() {
                    Some(first) => format!("{}{}", first.to_uppercase(), chars.collect::<String>().to_lowercase()),
                    None => String::new(),
                }
            }).collect::<Vec<_>>().join(" ");
            Ok(Value::String(Rc::new(result)))
        }
        "تجزئة" => {
            if args.is_empty() { return Err(RuntimeError::new("تجزئة يتطلب نصاً")); }
            let s = match &args[0] { Value::String(s) => s.as_str(), _ => return Err(RuntimeError::new("المعامل يجب ان يكون نصاً")) };
            let chunk_size = match args.get(1) { Some(Value::Integer(n)) => *n as usize, _ => 1 };
            if chunk_size == 0 { return Err(RuntimeError::new("حجم القطعة لا يمكن ان يكون صفر")); }
            let chunks: Vec<Value> = s.chars().collect::<Vec<char>>().chunks(chunk_size)
                .map(|c| Value::String(Rc::new(c.iter().collect())))
                .collect();
            Ok(Value::List(SharedList::new(chunks)))
        }
        "تحويل_لارقام" => {
            if args.is_empty() { return Err(RuntimeError::new("تحويل_لارقام يتطلب نصاً")); }
            let s = match &args[0] { Value::String(s) => s.as_str(), _ => return Err(RuntimeError::new("المعامل يجب ان يكون نصاً")) };
            let base = match args.get(1) { Some(Value::Integer(n)) => *n as u32, _ => 10 };
            match i64::from_str_radix(s, base) {
                Ok(n) => Ok(Value::Integer(n)),
                Err(_) => Err(RuntimeError::new(format!("لا يمكن تحويل '{}' لارقام بالنظام {}", s, base))),
            }
        }
        "تحويل_من_ارقام" => {
            if args.is_empty() { return Err(RuntimeError::new("تحويل_من_ارقام يتطلب عدداً")); }
            let n = match &args[0] { Value::Integer(i) => *i, _ => return Err(RuntimeError::new("المعامل يجب ان يكون صحيحاً")) };
            let base = match args.get(1) { Some(Value::Integer(b)) => *b as u32, _ => 10 };
            Ok(Value::String(Rc::new(match base {
                2 => format!("{:b}", n),
                8 => format!("{:o}", n),
                16 => format!("{:x}", n),
                _ => format!("{}", n),
            })))
        }
        "تنسيق" => {
            if args.is_empty() { return Err(RuntimeError::new("تنسيق يتطلب نصاً")); }
            let fmt = match &args[0] { Value::String(s) => s.as_str(), _ => return Err(RuntimeError::new("القالب يجب ان يكون نصاً")) };
            let mut result = fmt.to_string();
            for (i, arg) in args.iter().skip(1).enumerate() {
                let placeholder = format!("{{{}}}", i);
                result = result.replace(&placeholder, &arg.to_string_value());
            }
            Ok(Value::String(Rc::new(result)))
        }
        "يحتوي_اي" => {
            if args.len() < 2 { return Err(RuntimeError::new("يحتوي_اي يتطلب نصاً وقائمة")); }
            let s = match &args[0] { Value::String(s) => s.as_str(), _ => return Err(RuntimeError::new("المعامل الاول يجب ان يكون نصاً")) };
            let patterns = match &args[1] { Value::List(l) => l.borrow(), _ => return Err(RuntimeError::new("المعامل الثاني يجب ان يكون مصفوفة")) };
            for p in patterns.iter() {
                let pat = match p { Value::String(ps) => ps.as_str(), _ => continue };
                if s.contains(pat) { return Ok(Value::Boolean(true)); }
            }
            Ok(Value::Boolean(false))
        }
        "تقطيع" => {
            if args.is_empty() { return Err(RuntimeError::new("تقطيع يتطلب نصاً")); }
            let s = match &args[0] { Value::String(s) => s.as_str(), _ => return Err(RuntimeError::new("المعامل يجب ان يكون نصاً")) };
            let sep = args.get(1).and_then(|v| match v { Value::String(ps) => Some(ps.as_str()), _ => None }).unwrap_or(" ");
            let keep_empty = args.get(2).map(|v| v.is_truthy()).unwrap_or(false);
            let parts: Vec<Value> = if keep_empty {
                s.split(sep).map(|p| Value::String(Rc::new(p.to_string()))).collect()
            } else {
                s.split(sep).filter(|p| !p.is_empty()).map(|p| Value::String(Rc::new(p.to_string()))).collect()
            };
            Ok(Value::List(SharedList::new(parts)))
        }

        // === Random Extended ===
        "اختيار" => {
            let list_arg = args.first().ok_or_else(|| RuntimeError::new("اختيار يتطلب مصفوفة"))?;
            match list_arg {
                Value::List(list) => {
                    let items = list.borrow();
                    if items.is_empty() { return Err(RuntimeError::new("اختيار من مصفوفة فارغة")); }
                    let idx = (rand_random() * items.len() as f64) as usize;
                    Ok(items[idx.min(items.len() - 1)].clone())
                }
                _ => Err(RuntimeError::new_typed("استثناء_نوع", "اختيار يتطلب مصفوفة")),
            }
        }
        "عينة" => {
            let list_arg = args.get(0).ok_or_else(|| RuntimeError::new("عينة تتطلب مصفوفة وعدها"))?;
            let count = match args.get(1) { Some(Value::Integer(n)) => *n as usize, _ => 1 };
            match list_arg {
                Value::List(list) => {
                    let items = list.borrow();
                    if items.is_empty() { return Ok(Value::List(SharedList::new(vec![]))); }
                    let n = count.min(items.len());
                    let mut indices: Vec<usize> = (0..items.len()).collect();
                    let mut result = Vec::with_capacity(n);
                    for i in 0..n {
                        let j = (rand_random() * (indices.len() - i) as f64) as usize;
                        let j = j.min(indices.len() - 1 - i);
                        indices.swap(i, i + j);
                        result.push(items[indices[i]].clone());
                    }
                    Ok(Value::List(SharedList::new(result)))
                }
                _ => Err(RuntimeError::new_typed("استثناء_نوع", "عينة تتطلب مصفوفة")),
            }
        }
        "خلط" => {
            let list_arg = args.first().ok_or_else(|| RuntimeError::new("خلط يتطلب مصفوفة"))?;
            match list_arg {
                Value::List(list) => {
                    let items = list.borrow().clone();
                    let mut indices: Vec<usize> = (0..items.len()).collect();
                    for i in (1..indices.len()).rev() {
                        let j = (rand_random() * (i + 1) as f64) as usize;
                        let j = j.min(i);
                        indices.swap(i, j);
                    }
                    let shuffled: Vec<Value> = indices.into_iter().map(|i| items[i].clone()).collect();
                    Ok(Value::List(SharedList::new(shuffled)))
                }
                _ => Err(RuntimeError::new_typed("استثناء_نوع", "خلط يتطلب مصفوفة")),
            }
        }
        "طبيعي" => {
            let mean = args.get(0).map(|v| match v { Value::Integer(i) => *i as f64, Value::Float(f) => *f, _ => 0.0 }).unwrap_or(0.0);
            let std_dev = args.get(1).map(|v| match v { Value::Integer(i) => *i as f64, Value::Float(f) => *f, _ => 1.0 }).unwrap_or(1.0);
            let u1 = rand_random();
            let u2 = rand_random();
            let z = (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos();
            Ok(Value::Float(mean + std_dev * z))
        }
        "برنولي" => {
            let p = args.first().map(|v| match v { Value::Integer(i) => *i as f64, Value::Float(f) => *f, _ => 0.5 }).unwrap_or(0.5);
            Ok(Value::Boolean(rand_random() < p))
        }
        "عشوائي_نطاق" => {
            let start = match args.get(0) { Some(Value::Integer(n)) => *n, _ => return Err(RuntimeError::new("عشوائي_نطاق يتطلب بداية ونهاية")) };
            let end = match args.get(1) { Some(Value::Integer(n)) => *n, _ => return Err(RuntimeError::new("عشوائي_نطاق يتطلب نهاية")) };
            if start >= end { return Err(RuntimeError::new("البداية يجب ان اقل من النهاية")); }
            let range = (end - start) as f64;
            let val = start + (rand_random() * range) as i64;
            Ok(Value::Integer(val))
        }

        // === JSON Extended ===
        "جميل" => {
            let val = args.first().ok_or_else(|| RuntimeError::new("جميل يتطلب قيمة"))?;
            let json_val = arabi_value_to_json(val)?;
            let pretty = serde_json::to_string_pretty(&json_val)
                .map_err(|e| RuntimeError::new(format!("خطا في JSON: {}", e)))?;
            Ok(Value::String(Rc::new(pretty)))
        }
        "تحقق" => {
            let s = args.first().ok_or_else(|| RuntimeError::new("تحقق يتطلب نصاً"))?;
            match s {
                Value::String(s) => {
                    match serde_json::from_str::<serde_json::Value>(s.as_ref()) {
                        Ok(_) => Ok(Value::Boolean(true)),
                        Err(_) => Ok(Value::Boolean(false)),
                    }
                }
                _ => Err(RuntimeError::new_typed("استثناء_نوع", "تحقق يتطلب نصاً")),
            }
        }

        // === File Operations ===
        "اقرا_ملف" => {
            let path = args.first().ok_or_else(|| RuntimeError::new("اقرا_ملف يتطلب مساراً"))?;
            match path {
                Value::String(s) => {
                    let content = std::fs::read_to_string(s.as_ref())
                        .map_err(|e| RuntimeError::new(format!("خطا في قراءة الملف '{}': {}", s, e)))?;
                    Ok(Value::String(Rc::new(content)))
                }
                _ => Err(RuntimeError::new_typed("استثناء_نوع", "اقرا_ملف يتطلب نصاً")),
            }
        }
        "اكتب_ملف" => {
            if args.len() < 2 { return Err(RuntimeError::new("اكتب_ملف يتطلب مساراً ونصاً")); }
            match (&args[0], &args[1]) {
                (Value::String(path), Value::String(content)) => {
                    std::fs::write(path.as_ref(), content.as_ref())
                        .map_err(|e| RuntimeError::new(format!("خطا في كتابة الملف '{}': {}", path, e)))?;
                    Ok(Value::Boolean(true))
                }
                _ => Err(RuntimeError::new_typed("استثناء_نوع", "اكتب_ملف يتطلب نصين (المسار والمحتوى)")),
            }
        }
        "اضف_ملف" => {
            if args.len() < 2 { return Err(RuntimeError::new("اضف_ملف يتطلب مساراً ونصاً")); }
            match (&args[0], &args[1]) {
                (Value::String(path), Value::String(content)) => {
                    use std::io::Write;
                    let mut file = std::fs::OpenOptions::new()
                        .create(true).append(true)
                        .open(path.as_ref())
                        .map_err(|e| RuntimeError::new(format!("خطا في فتح الملف '{}': {}", path, e)))?;
                    file.write_all(content.as_ref().as_bytes())
                        .map_err(|e| RuntimeError::new(format!("خطا في كتابة الملف '{}': {}", path, e)))?;
                    Ok(Value::Boolean(true))
                }
                _ => Err(RuntimeError::new_typed("استثناء_نوع", "اضف_ملف يتطلب نصين (المسار والمحتوى)")),
            }
        }
        "يوجد" => {
            let path = args.first().ok_or_else(|| RuntimeError::new("يوجد يتطلب مساراً"))?;
            match path {
                Value::String(s) => Ok(Value::Boolean(std::path::Path::new(s.as_ref()).exists())),
                _ => Err(RuntimeError::new_typed("استثناء_نوع", "يوجد يتطلب نصاً")),
            }
        }
        "ملف_الحجم" => {
            let path = args.first().ok_or_else(|| RuntimeError::new("ملف_الحجم يتطلب مساراً"))?;
            match path {
                Value::String(s) => {
                    let meta = std::fs::metadata(s.as_ref())
                        .map_err(|e| RuntimeError::new(format!("خطا في قراءة معلومات '{}': {}", s, e)))?;
                    Ok(Value::Integer(meta.len() as i64))
                }
                _ => Err(RuntimeError::new_typed("استثناء_نوع", "ملف_الحجم يتطلب نصاً")),
            }
        }
        "التمس" => {
            if args.is_empty() { return Err(RuntimeError::new("التمس يتطلب نمطاً")); }
            let pattern = match &args[0] { Value::String(s) => s.as_str(), _ => return Err(RuntimeError::new("النمط يجب ان يكون نصاً")) };
            let dir = args.get(1).and_then(|v| match v { Value::String(s) => Some(s.as_str()), _ => None }).unwrap_or(".");
            let entries = std::fs::read_dir(dir)
                .map_err(|e| RuntimeError::new(format!("خطا في قراءة المجلد: {}", e)))?;
            let matches: Vec<Value> = entries.filter_map(|e| e.ok()).filter(|e| {
                let name = e.file_name().to_string_lossy().to_string();
                name.contains(pattern)
            }).map(|e| Value::String(Rc::new(e.file_name().to_string_lossy().to_string()))).collect();
            Ok(Value::List(SharedList::new(matches)))
        }

        // === File Utility Functions ===
        "احذف_ملف" => {
            let path = args.first().ok_or_else(|| RuntimeError::new("احذف_ملف يتطلب مساراً"))?;
            match path {
                Value::String(s) => {
                    let p = std::path::Path::new(s.as_ref());
                    if p.is_dir() {
                        std::fs::remove_dir_all(s.as_ref())
                            .map_err(|e| RuntimeError::new(format!("خطا في حذف المجلد '{}': {}", s, e)))?;
                    } else {
                        std::fs::remove_file(s.as_ref())
                            .map_err(|e| RuntimeError::new(format!("خطا في حذف الملف '{}': {}", s, e)))?;
                    }
                    Ok(Value::Boolean(true))
                }
                _ => Err(RuntimeError::new_typed("استثناء_نوع", "احذف_ملف يتطلب نصاً")),
            }
        }
        "اقرا_اسطر" => {
            let path = args.first().ok_or_else(|| RuntimeError::new("اقرا_اسطر يتطلب مساراً"))?;
            match path {
                Value::String(s) => {
                    let content = std::fs::read_to_string(s.as_ref())
                        .map_err(|e| RuntimeError::new(format!("خطا في قراءة الملف '{}': {}", s, e)))?;
                    let lines: Vec<Value> = content.lines()
                        .map(|l| Value::String(Rc::new(l.to_string())))
                        .collect();
                    Ok(Value::List(SharedList::new(lines)))
                }
                _ => Err(RuntimeError::new_typed("استثناء_نوع", "اقرا_اسطر يتطلب نصاً")),
            }
        }
        "اسم_الملف" => {
            let path = args.first().ok_or_else(|| RuntimeError::new("اسم_الملف يتطلب مساراً"))?;
            match path {
                Value::String(s) => {
                    let p = std::path::Path::new(s.as_ref());
                    let name = p.file_name().unwrap_or_default().to_string_lossy().to_string();
                    Ok(Value::String(Rc::new(name)))
                }
                _ => Err(RuntimeError::new_typed("استثناء_نوع", "اسم_الملف يتطلب نصاً")),
            }
        }
        "امتداد_ملف" => {
            let path = args.first().ok_or_else(|| RuntimeError::new("امتداد_ملف يتطلب مساراً"))?;
            match path {
                Value::String(s) => {
                    let p = std::path::Path::new(s.as_ref());
                    let ext = p.extension().unwrap_or_default().to_string_lossy().to_string();
                    Ok(Value::String(Rc::new(ext)))
                }
                _ => Err(RuntimeError::new_typed("استثناء_نوع", "امتداد_ملف يتطلب نصاً")),
            }
        }
        "المسار_المجلد" => {
            let path = args.first().ok_or_else(|| RuntimeError::new("المسار_المجلد يتطلب مساراً"))?;
            match path {
                Value::String(s) => {
                    let p = std::path::Path::new(s.as_ref());
                    let dir = p.parent().unwrap_or(std::path::Path::new(".")).to_string_lossy().to_string();
                    Ok(Value::String(Rc::new(dir)))
                }
                _ => Err(RuntimeError::new_typed("استثناء_نوع", "المسار_المجلد يتطلب نصاً")),
            }
        }

        // === Dict Operations ===
        "مفاتيح" => {
            let obj = args.first().ok_or_else(|| RuntimeError::new("مفاتيح يتطلب فهرساً"))?;
            match obj {
                Value::Dict(d) => {
                    let pairs = d.borrow();
                    let keys: Vec<Value> = pairs.iter().map(|(k, _)| k.clone()).collect();
                    Ok(Value::List(SharedList::new(keys)))
                }
                _ => Err(RuntimeError::new_typed("استثناء_نوع", "مفاتيح يتطلب فهرساً")),
            }
        }
        "قيم" => {
            let obj = args.first().ok_or_else(|| RuntimeError::new("قيم يتطلب فهرساً"))?;
            match obj {
                Value::Dict(d) => {
                    let pairs = d.borrow();
                    let vals: Vec<Value> = pairs.iter().map(|(_, v)| v.clone()).collect();
                    Ok(Value::List(SharedList::new(vals)))
                }
                _ => Err(RuntimeError::new_typed("استثناء_نوع", "قيم يتطلب فهرساً")),
            }
        }
        "ازواج" => {
            let obj = args.first().ok_or_else(|| RuntimeError::new("ازواج يتطلب فهرساً"))?;
            match obj {
                Value::Dict(d) => {
                    let pairs = d.borrow();
                    let zipped: Vec<Value> = pairs.iter().map(|(k, v)| {
                        Value::List(SharedList::new(vec![k.clone(), v.clone()]))
                    }).collect();
                    Ok(Value::List(SharedList::new(zipped)))
                }
                _ => Err(RuntimeError::new_typed("استثناء_نوع", "ازواج يتطلب فهرساً")),
            }
        }
        "يحتوي_المفتاح" => {
            if args.len() < 2 { return Err(RuntimeError::new("يحتوي_المفتاح يتطلب فهرساً ومفتاحاً")); }
            match &args[0] {
                Value::Dict(d) => {
                    let idx = d.index.borrow();
                    Ok(Value::Boolean(idx.contains_key(&args[1])))
                }
                _ => Err(RuntimeError::new_typed("استثناء_نوع", "يحتوي_المفتاح يتطلب فهرساً")),
            }
        }
        "ادمج_فهرس" => {
            if args.len() < 2 { return Err(RuntimeError::new("ادمج_فهرس يتطلب فهرسين")); }
            match (&args[0], &args[1]) {
                (Value::Dict(a), Value::Dict(b)) => {
                    let mut result = Vec::new();
                    for (k, v) in a.borrow().iter() { result.push((k.clone(), v.clone())); }
                    for (k, v) in b.borrow().iter() { result.push((k.clone(), v.clone())); }
                    Ok(Value::Dict(SharedDict::new(result)))
                }
                _ => Err(RuntimeError::new_typed("استثناء_نوع", "ادمج_فهرس يتطلب فهرسين")),
            }
        }

        // === OS Extended ===
        "انضم" => {
            let mut result = std::path::PathBuf::new();
            for arg in args {
                match arg {
                    Value::String(s) => result.push(s.as_ref()),
                    _ => return Err(RuntimeError::new_typed("استثناء_نوع", "انضم يتطلب نصوصاً")),
                }
            }
            Ok(Value::String(Rc::new(result.to_string_lossy().to_string())))
        }
        "مسار_مطلق" => {
            let p = args.first().ok_or_else(|| RuntimeError::new("مسار_مطلق يتطلب مساراً"))?;
            match p {
                Value::String(s) => {
                    let path = std::path::Path::new(s.as_ref());
                    let abs = if path.is_absolute() { path.to_path_buf() } else { std::env::current_dir().unwrap_or_default().join(path) };
                    Ok(Value::String(Rc::new(abs.to_string_lossy().to_string())))
                }
                _ => Err(RuntimeError::new_typed("استثناء_نوع", "مسار_مطلق يتطلب نصاً")),
            }
        }
        "نسخ" => {
            let src = args.get(0).ok_or_else(|| RuntimeError::new("نسخ يتطلب مصدراً ومقصداً"))?;
            let dst = args.get(1).ok_or_else(|| RuntimeError::new("نسخ يتطلب مصدراً ومقصداً"))?;
            match (src, dst) {
                (Value::String(s), Value::String(d)) => {
                    std::fs::copy(s.as_ref(), d.as_ref())
                        .map_err(|e| RuntimeError::new(format!("خطا في النسخ: {}", e)))?;
                    Ok(Value::Null)
                }
                _ => Err(RuntimeError::new_typed("استثناء_نوع", "نسخ يتطلب نصين")),
            }
        }
        "نقل" => {
            let src = args.get(0).ok_or_else(|| RuntimeError::new("نقل يتطلب مصدراً ومقصداً"))?;
            let dst = args.get(1).ok_or_else(|| RuntimeError::new("نقل يتطلب مصدراً ومقصداً"))?;
            match (src, dst) {
                (Value::String(s), Value::String(d)) => {
                    std::fs::rename(s.as_ref(), d.as_ref())
                        .map_err(|e| RuntimeError::new(format!("خطا في النقل: {}", e)))?;
                    Ok(Value::Null)
                }
                _ => Err(RuntimeError::new_typed("استثناء_نوع", "نقل يتطلب نصين")),
            }
        }
        "قائمة_مجلد" => {
            let p = args.first().ok_or_else(|| RuntimeError::new("قائمة_مجلد يتطلب مساراً"))?;
            match p {
                Value::String(s) => {
                    let entries = std::fs::read_dir(s.as_ref())
                        .map_err(|e| RuntimeError::new(format!("خطا في قراءة المجلد: {}", e)))?;
                    let names: Vec<Value> = entries.filter_map(|e| e.ok()).map(|e| {
                        let name = e.file_name().to_string_lossy().to_string();
                        Value::String(Rc::new(name))
                    }).collect();
                    Ok(Value::List(SharedList::new(names)))
                }
                _ => Err(RuntimeError::new_typed("استثناء_نوع", "قائمة_مجلد يتطلب نصاً")),
            }
        }
        "مشي" => {
            let p = args.first().ok_or_else(|| RuntimeError::new("مشي يتطلب مساراً"))?;
            match p {
                Value::String(s) => {
                    let mut result = Vec::new();
                    let walk_dir = std::path::Path::new(s.as_ref());
                    if let Ok(entries) = std::fs::read_dir(walk_dir) {
                        for entry in entries.filter_map(|e| e.ok()) {
                            let path = entry.path();
                            let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
                            let is_dir = path.is_dir();
                            let size = if is_dir { 0 } else { entry.metadata().map(|m| m.len() as i64).unwrap_or(0) };
                            result.push(Value::Tuple(Rc::new(vec![
                                Value::String(Rc::new(name)),
                                Value::Boolean(is_dir),
                                Value::Integer(size),
                            ])));
                        }
                    }
                    Ok(Value::List(SharedList::new(result)))
                }
                _ => Err(RuntimeError::new_typed("استثناء_نوع", "مشي يتطلب نصاً")),
            }
        }
        "متغير_بيئي" => {
            let name = args.first().ok_or_else(|| RuntimeError::new("متغير_بيئي يتطلب اسماً"))?;
            match name {
                Value::String(s) => {
                    match std::env::var(s.as_ref()) {
                        Ok(val) => Ok(Value::String(Rc::new(val))),
                        Err(_) => Ok(Value::Null),
                    }
                }
                _ => Err(RuntimeError::new_typed("استثناء_نوع", "متغير_بيئي يتطلب نصاً")),
            }
        }

        "احذف_عنصر" => {
            let obj = args.first().ok_or_else(|| RuntimeError::new("احذف_عنصر يتطلب مصفوفة"))?;
            match obj {
                Value::List(items) => {
                    let idx = if args.len() > 1 {
                        match &args[1] {
                            Value::Integer(n) => {
                                let len = items.borrow().len() as i64;
                                if *n < 0 { (len + n) as usize } else { *n as usize }
                            }
                            _ => return Err(RuntimeError::new_typed("استثناء_نوع", "احذف_عنصر يتطلب فهرساً صحيحاً")),
                        }
                    } else {
                        items.borrow().len().saturating_sub(1)
                    };
                    let mut list = items.borrow_mut();
                    if idx >= list.len() {
                        return Err(RuntimeError::new_typed("استثناء_نطاق", "الفهرس خارج النطاق"));
                    }
                    Ok(list.remove(idx))
                }
                _ => Err(RuntimeError::new_typed("استثناء_نوع", "احذف_عنصر يتطلب مصفوفة")),
            }
        }

        "احطظ" => {
            if args.len() < 2 {
                return Err(RuntimeError::new("احطظ يتطلب مصفوفة وقيمة"));
            }
            match &args[0] {
                Value::List(items) => {
                    items.borrow_mut().push(args[1].clone());
                    Ok(args[1].clone())
                }
                _ => Err(RuntimeError::new_typed("استثناء_نوع", "احطظ يتطلب مصفوفة")),
            }
        }

        "ادخل_في" => {
            if args.len() < 3 {
                return Err(RuntimeError::new("ادخل_في يتطلب مصفوفة وموضع وقيمة"));
            }
            match &args[0] {
                Value::List(items) => {
                    let idx = match &args[1] {
                        Value::Integer(n) => {
                            let len = items.borrow().len() as i64;
                            if *n < 0 { (len + n) as usize } else { *n as usize }
                        }
                        _ => return Err(RuntimeError::new_typed("استثناء_نوع", "ادخل_في يتطلب موضعاً صحيحاً")),
                    };
                    let mut list = items.borrow_mut();
                    if idx > list.len() {
                        return Err(RuntimeError::new_typed("استثناء_نطاق", "الموضع خارج النطاق"));
                    }
                    list.insert(idx, args[2].clone());
                    Ok(Value::Null)
                }
                _ => Err(RuntimeError::new_typed("استثناء_نوع", "ادخل_في يتطلب مصفوفة")),
            }
        }

        "احذف_قيمة" => {
            if args.len() < 2 {
                return Err(RuntimeError::new("احذف_قيمة يتطلب مصفوفة وقيمة"));
            }
            match &args[0] {
                Value::List(items) => {
                    let mut list = items.borrow_mut();
                    let pos = list.iter().position(|v| v == &args[1]);
                    match pos {
                        Some(i) => { list.remove(i); Ok(Value::Boolean(true)) }
                        None => Ok(Value::Boolean(false)),
                    }
                }
                _ => Err(RuntimeError::new_typed("استثناء_نوع", "احذف_قيمة يتطلب مصفوفة")),
            }
        }

        "نفذ" => {
            let cmd = args.first().ok_or_else(|| RuntimeError::new("نفذ يتطلب أمراً"))?;
            let cmd_str = match cmd {
                Value::String(s) => s.as_ref().clone(),
                _ => return Err(RuntimeError::new_typed("استثناء_نوع", "نفذ يتطلب نصاً")),
            };
            let output = if cfg!(windows) {
                std::process::Command::new("cmd")
                    .arg("/C")
                    .arg(&cmd_str)
                    .output()
            } else {
                std::process::Command::new("sh")
                    .arg("-c")
                    .arg(&cmd_str)
                    .output()
            }.map_err(|e| RuntimeError::new(format!("فشل تنفيذ الأمر: {}", e)))?;
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            let code = output.status.code().unwrap_or(-1);
            Ok(Value::Tuple(Rc::new(vec![
                Value::Integer(code as i64),
                Value::String(stdout.into()),
                Value::String(stderr.into()),
            ])))
        }

        "اخرج" => {
            let code = if let Some(c) = args.first() {
                match c {
                    Value::Integer(n) => *n as i32,
                    _ => 0,
                }
            } else {
                0
            };
            std::process::exit(code);
        }

        // === Functional Programming / Higher-Order ===

        "مسطح" => {
            let obj = args.first().ok_or_else(|| RuntimeError::new("مسطح يتطلب مصفوفة"))?;
            match obj {
                Value::List(items) => {
                    let mut result = Vec::new();
                    fn flatten(items: &[Value], out: &mut Vec<Value>) {
                        for item in items {
                            if let Value::List(inner) = item {
                                flatten(&inner.borrow(), out);
                            } else {
                                out.push(item.clone());
                            }
                        }
                    }
                    flatten(&items.borrow(), &mut result);
                    Ok(Value::List(SharedList::new(result)))
                }
                _ => Err(RuntimeError::new_typed("استثناء_نوع", "مسطح يتطلب مصفوفة")),
            }
        }

        "ضخم" => {
            if args.len() < 2 {
                return Err(RuntimeError::new("ضخم يتطلب دالة ومصفوفة"));
            }
            let func = &args[0];
            let iterable = &args[1];
            let items: Vec<Value> = match iterable {
                Value::List(l) => l.borrow().clone(),
                Value::Tuple(t) => t.as_ref().clone(),
                _ => return Err(RuntimeError::new_typed("استثناء_نوع", "ضخم يتطلب قائمة او مترابطة")),
            };
            let mut result = Vec::new();
            for item in items {
                let mapped = func.call(&[item], &[], vm, module)?;
                match mapped {
                    Value::List(l) => {
                        for v in l.borrow().iter() {
                            result.push(v.clone());
                        }
                    }
                    _ => result.push(mapped),
                }
            }
            Ok(Value::List(SharedList::new(result)))
        }

        "ادمج_فهرس_بـ" => {
            if args.len() < 2 {
                return Err(RuntimeError::new("ادمج_فهرس_بـ يتطلب مصفوفتي مفاتيح وقيم"));
            }
            let keys = match &args[0] {
                Value::List(l) => l.borrow().clone(),
                Value::Tuple(t) => t.as_ref().clone(),
                _ => return Err(RuntimeError::new_typed("استثناء_نوع", "ادمج_فهرس_بـ يتطلب مصفوفة مفاتيح")),
            };
            let vals = match &args[1] {
                Value::List(l) => l.borrow().clone(),
                Value::Tuple(t) => t.as_ref().clone(),
                _ => return Err(RuntimeError::new_typed("استثناء_نوع", "ادمج_فهرس_بـ يتطلب مصفوفة قيم")),
            };
            if keys.len() != vals.len() {
                return Err(RuntimeError::new("ادمج_فهرس_بـ يتطلب مصفوفتين متساويتين الطول"));
            }
            let mut map = Vec::new();
            for (k, v) in keys.into_iter().zip(vals.into_iter()) {
                map.push((k, v));
            }
            Ok(Value::Dict(SharedDict::new(map)))
        }

        "تجميع" => {
            if args.len() < 2 {
                return Err(RuntimeError::new("تجميع يتطلب دالة ومصفوفة"));
            }
            let func = &args[0];
            let items: Vec<Value> = match &args[1] {
                Value::List(l) => l.borrow().clone(),
                Value::Tuple(t) => t.as_ref().clone(),
                _ => return Err(RuntimeError::new_typed("استثناء_نوع", "تجميع يتطلب قائمة")),
            };
            let mut groups: std::collections::HashMap<String, Vec<Value>> = std::collections::HashMap::new();
            for item in items {
                let key = func.call(&[item.clone()], &[], vm, module)?;
                let key_str = match &key {
                    Value::String(s) => s.as_ref().clone(),
                    Value::Integer(n) => n.to_string(),
                    Value::Float(f) => f.to_string(),
                    Value::Boolean(b) => b.to_string(),
                    Value::Null => "عدم".to_string(),
                    other => format!("{:?}", other),
                };
                groups.entry(key_str).or_default().push(item);
            }
            let mut result = std::collections::HashMap::new();
            for (k, v) in groups {
                result.insert(Value::String(k.into()), Value::List(SharedList::new(v)));
            }
            Ok(Value::Dict(SharedDict::new(result.into_iter().collect())))
        }

        "عدد_تكرار" => {
            if args.is_empty() {
                return Err(RuntimeError::new("عدد_تكرار يتطلب مصفوفة"));
            }
            if args.len() == 1 {
                let items: Vec<Value> = match &args[0] {
                    Value::List(l) => l.borrow().clone(),
                    Value::Tuple(t) => t.as_ref().clone(),
                    _ => return Err(RuntimeError::new_typed("استثناء_نوع", "عدد_تكرار يتطلب قائمة")),
                };
                let mut counts: std::collections::HashMap<String, i64> = std::collections::HashMap::new();
                for item in items {
                    let key = match &item {
                        Value::String(s) => s.as_ref().clone(),
                        Value::Integer(n) => n.to_string(),
                        Value::Float(f) => f.to_string(),
                        Value::Boolean(b) => b.to_string(),
                        Value::Null => "عدم".to_string(),
                        other => format!("{:?}", other),
                    };
                    *counts.entry(key).or_insert(0) += 1;
                }
                let mut result = Vec::new();
                for (k, v) in counts {
                    result.push((Value::String(k.into()), Value::Integer(v)));
                }
                return Ok(Value::Dict(SharedDict::new(result)));
            }
            let func = &args[0];
            let items: Vec<Value> = match &args[1] {
                Value::List(l) => l.borrow().clone(),
                Value::Tuple(t) => t.as_ref().clone(),
                _ => return Err(RuntimeError::new_typed("استثناء_نوع", "عدد_تكرار يتطلب قائمة")),
            };
            let mut counts: std::collections::HashMap<String, i64> = std::collections::HashMap::new();
            for item in items {
                let key = func.call(&[item], &[], vm, module)?;
                let key_str = match &key {
                    Value::String(s) => s.as_ref().clone(),
                    Value::Integer(n) => n.to_string(),
                    Value::Float(f) => f.to_string(),
                    Value::Boolean(b) => b.to_string(),
                    Value::Null => "عدم".to_string(),
                    other => format!("{:?}", other),
                };
                *counts.entry(key_str).or_insert(0) += 1;
            }
            let mut result = Vec::new();
            for (k, v) in counts {
                result.push((Value::String(k.into()), Value::Integer(v)));
            }
            Ok(Value::Dict(SharedDict::new(result)))
        }

        "تجزئة_قائمة" => {
            if args.len() < 2 {
                return Err(RuntimeError::new("تجزئة_قائمة تتطلب مصفوفة وحجم"));
            }
            let items = match &args[0] {
                Value::List(l) => l.borrow().clone(),
                _ => return Err(RuntimeError::new_typed("استثناء_نوع", "تجزئة_قائمة تتطلب مصفوفة")),
            };
            let chunk_size = match &args[1] {
                Value::Integer(n) if *n > 0 => *n as usize,
                _ => return Err(RuntimeError::new_typed("استثناء_نوع", "تجزئة_قائمة تتطلب حجماً صحيحياً موجباً")),
            };
            let mut chunks = Vec::new();
            for chunk in items.chunks(chunk_size) {
                chunks.push(Value::List(SharedList::new(chunk.to_vec())));
            }
            Ok(Value::List(SharedList::new(chunks)))
        }

        "افصل" => {
            if args.len() < 2 {
                return Err(RuntimeError::new("افصل يتطلب دالة ومصفوفة"));
            }
            let func = &args[0];
            let items: Vec<Value> = match &args[1] {
                Value::List(l) => l.borrow().clone(),
                Value::Tuple(t) => t.as_ref().clone(),
                _ => return Err(RuntimeError::new_typed("استثناء_نوع", "افصل يتطلب قائمة")),
            };
            let mut matching = Vec::new();
            let mut not_matching = Vec::new();
            for item in items {
                let result = func.call(&[item.clone()], &[], vm, module)?;
                if result.is_truthy() {
                    matching.push(item);
                } else {
                    not_matching.push(item);
                }
            }
            Ok(Value::List(SharedList::new(vec![
                Value::List(SharedList::new(matching)),
                Value::List(SharedList::new(not_matching)),
            ])))
        }

        // === HTTP builtins ===
        "طلب" => {
            let url = args.first().ok_or_else(|| RuntimeError::new("طلب يتطلب رابطاً"))?;
            match url {
                Value::String(u) => {
                    let resp = ureq::get(u).call()
                        .map_err(|e| RuntimeError::new_typed("استثناء_نوع", format!("فشل الطلب: {}", e)))?;
                    let body = resp.into_string()
                        .map_err(|e| RuntimeError::new_typed("استثناء_نوع", format!("فشل قراءة الاستجابة: {}", e)))?;
                    Ok(Value::String(Rc::new(body)))
                }
                _ => Err(RuntimeError::new_typed("استثناء_نوع", "طلب يتطلب نصاً (الرابط)")),
            }
        }
        "طلب_نص" => {
            let url = args.first().ok_or_else(|| RuntimeError::new("طلب_نص يتطلب رابطاً"))?;
            match url {
                Value::String(u) => {
                    let resp = ureq::get(u).call()
                        .map_err(|e| RuntimeError::new_typed("استثناء_نوع", format!("فشل الطلب: {}", e)))?;
                    let body = resp.into_string()
                        .map_err(|e| RuntimeError::new_typed("استثناء_نوع", format!("فشل قراءة الاستجابة: {}", e)))?;
                    let code = 200;
                    let result = vec![
                        (Value::String(Rc::new("محتوى".to_string())), Value::String(Rc::new(body))),
                        (Value::String(Rc::new("حالة".to_string())), Value::Integer(code)),
                    ];
                    Ok(Value::Dict(SharedDict::new(result)))
                }
                _ => Err(RuntimeError::new_typed("استثناء_نوع", "طلب_نص يتطلب نصاً (الرابط)")),
            }
        }
        "طلب_كائن" => {
            let url = args.first().ok_or_else(|| RuntimeError::new("طلب_كائن يتطلب رابطاً"))?;
            match url {
                Value::String(u) => {
                    let resp = ureq::get(u).call()
                        .map_err(|e| RuntimeError::new_typed("استثناء_نوع", format!("فشل الطلب: {}", e)))?;
                    let body = resp.into_string()
                        .map_err(|e| RuntimeError::new_typed("استثناء_نوع", format!("فشل قراءة الاستجابة: {}", e)))?;
                    let json_val: serde_json::Value = serde_json::from_str(&body)
                        .map_err(|e| RuntimeError::new_typed("استثناء_نوع", format!("JSON غير صحيح: {}", e)))?;
                    Ok(json_to_arabi(json_val))
                }
                _ => Err(RuntimeError::new_typed("استثناء_نوع", "طلب_json يتطلب نصاً (الرابط)")),
            }
        }
        "طلب_ارسال" => {
            let url = args.first().ok_or_else(|| RuntimeError::new("طلب_ارسال يتطلب رابطاً"))?;
            let data = args.get(1).ok_or_else(|| RuntimeError::new("طلب_ارسال يتطلب بيانات"))?;
            match (url, data) {
                (Value::String(u), Value::String(d)) => {
                    let resp = ureq::post(u)
                        .set("Content-Type", "application/json")
                        .send_string(d)
                        .map_err(|e| RuntimeError::new_typed("استثناء_نوع", format!("فشل الطلب: {}", e)))?;
                    let body = resp.into_string()
                        .map_err(|e| RuntimeError::new_typed("استثناء_نوع", format!("فشل قراءة الاستجابة: {}", e)))?;
                    Ok(Value::String(Rc::new(body)))
                }
                _ => Err(RuntimeError::new_typed("استثناء_نوع", "طلب_ارسال يتطلب رابطاً وبيانات")),
            }
        }

        // === Additional Math builtins ===
        "حاصل_ضرب" => {
            let lst = args.first().ok_or_else(|| RuntimeError::new("حاصل_ضرب تتطلب مصفوفة"))?;
            match lst {
                Value::List(l) => {
                    let items = l.borrow();
                    let mut product = 1.0_f64;
                    let mut all_int = true;
                    let mut product_int: i64 = 1;
                    for item in items.iter() {
                        match item {
                            Value::Integer(n) => { product_int = product_int.checked_mul(*n).unwrap_or(0); }
                            Value::Float(f) => { all_int = false; product *= f; }
                            _ => return Err(RuntimeError::new_typed("استثناء_نوع", "حاصل_ضرب تتطلب عدداً")),
                        }
                    }
                    if all_int {
                        Ok(Value::Integer(product_int))
                    } else {
                        for item in items.iter() {
                            if let Value::Integer(n) = item { product *= *n as f64; }
                        }
                        Ok(Value::Float(product))
                    }
                }
                _ => Err(RuntimeError::new_typed("استثناء_نوع", "حاصل_ضرب تتطلب مصفوفة")),
            }
        }
        "نسبة" => {
            let val = args.first().ok_or_else(|| RuntimeError::new("نسبة تتطلب قيمة"))?;
            let total = args.get(1).ok_or_else(|| RuntimeError::new("نسبة تتطلب الاجمالي"))?;
            match (val, total) {
                (Value::Integer(a), Value::Integer(b)) => {
                    if *b == 0 { return Ok(Value::Float(0.0)); }
                    Ok(Value::Float(*a as f64 / *b as f64 * 100.0))
                }
                (Value::Float(a), Value::Float(b)) => {
                    if *b == 0.0 { return Ok(Value::Float(0.0)); }
                    Ok(Value::Float(a / b * 100.0))
                }
                _ => Err(RuntimeError::new_typed("استثناء_نوع", "نسبة تتطلب عددين")),
            }
        }
        "تقريب_ل" => {
            let val = args.first().ok_or_else(|| RuntimeError::new("تقريب_ل تتطلب قيمة"))?;
            let decimals = args.get(1).and_then(|v| match v { Value::Integer(n) => Some(*n as u32), _ => None }).unwrap_or(0);
            match val {
                Value::Float(f) => {
                    let factor = 10_f64.powi(decimals as i32);
                    Ok(Value::Float((f * factor).round() / factor))
                }
                Value::Integer(n) => Ok(Value::Integer(*n)),
                _ => Err(RuntimeError::new_typed("استثناء_نوع", "تقريب_ل تتطلب عدداً")),
            }
        }
        "علامة" => {
            let val = args.first().ok_or_else(|| RuntimeError::new("علامة تتطلب عدداً"))?;
            match val {
                Value::Integer(n) => Ok(Value::Integer(if *n > 0 { 1 } else if *n < 0 { -1 } else { 0 })),
                Value::Float(f) => Ok(Value::Integer(if *f > 0.0 { 1 } else if *f < 0.0 { -1 } else { 0 })),
                _ => Err(RuntimeError::new_typed("استثناء_نوع", "علامة تتطلب عدداً")),
            }
        }

        // === Encoding builtins ===
        "تشفير_64" => {
            let s = args.first().ok_or_else(|| RuntimeError::new("تشفير_64 يتطلب نصاً"))?;
            match s {
                Value::String(t) => {
                    use base64::Engine;
                    let encoded = base64::engine::general_purpose::STANDARD.encode(t.as_bytes());
                    Ok(Value::String(Rc::new(encoded)))
                }
                _ => Err(RuntimeError::new_typed("استثناء_نوع", "تشفير_64 يتطلب نصاً")),
            }
        }
        "فك_تشفير_64" => {
            let s = args.first().ok_or_else(|| RuntimeError::new("فك_تشفير_64 يتطلب نصاً"))?;
            match s {
                Value::String(t) => {
                    use base64::Engine;
                    let decoded = base64::engine::general_purpose::STANDARD.decode(t.as_bytes())
                        .map_err(|e| RuntimeError::new_typed("استثناء_نوع", format!("تشفير 64 غير صحيح: {}", e)))?;
                    Ok(Value::String(Rc::new(String::from_utf8(decoded).unwrap_or_default())))
                }
                _ => Err(RuntimeError::new_typed("استثناء_نوع", "فك_تشفير_64 يتطلب نصاً")),
            }
        }
        "تشفير_سداسي" => {
            let s = args.first().ok_or_else(|| RuntimeError::new("تشفير_سداسي يتطلب نصاً"))?;
            match s {
                Value::String(t) => {
                    let encoded: String = t.bytes().map(|b| format!("{:02x}", b)).collect();
                    Ok(Value::String(Rc::new(encoded)))
                }
                _ => Err(RuntimeError::new_typed("استثناء_نوع", "تشفير_سداسي يتطلب نصاً")),
            }
        }
        "فك_تشفير_سداسي" => {
            let s = args.first().ok_or_else(|| RuntimeError::new("فك_تشفير_سداسي يتطلب نصاً"))?;
            match s {
                Value::String(t) => {
                    let decoded: Vec<u8> = (0..t.len())
                        .step_by(2)
                        .filter_map(|i| u8::from_str_radix(&t[i..i + 2], 16).ok())
                        .collect();
                    Ok(Value::String(Rc::new(String::from_utf8(decoded).unwrap_or_default())))
                }
                _ => Err(RuntimeError::new_typed("استثناء_نوع", "فك_تشفير_سداسي يتطلب نصاً")),
            }
        }

        // === URL encoding ===
        "تشفير_رابط" => {
            let s = args.first().ok_or_else(|| RuntimeError::new("تشفير_رابط يتطلب نصاً"))?;
            match s {
                Value::String(t) => {
                    let encoded: String = t.bytes().map(|b| {
                        if b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || b == b'.' || b == b'~' {
                            char::from(b).to_string()
                        } else {
                            format!("%{:02X}", b)
                        }
                    }).collect();
                    Ok(Value::String(Rc::new(encoded)))
                }
                _ => Err(RuntimeError::new_typed("استثناء_نوع", "تشفير_رابط يتطلب نصاً")),
            }
        }

        // === Collection builtins ===
        "مفتاح_اكبر" => {
            let d = args.first().ok_or_else(|| RuntimeError::new("مفتاح_اكبر يتطلب مترابطة"))?;
            match d {
                Value::Dict(dict) => {
                    let borrow = dict.borrow();
                    let mut max_key: Option<Value> = None;
                    for (k, _) in borrow.iter() {
                        max_key = Some(match &max_key {
                            Some(mk) if mk.to_string_value() >= k.to_string_value() => mk.clone(),
                            _ => k.clone(),
                        });
                    }
                    Ok(max_key.unwrap_or(Value::Null))
                }
                _ => Err(RuntimeError::new_typed("استثناء_نوع", "مفتاح_اكبر تتطلب مترابطة")),
            }
        }
        "مفتاح_اصغر" => {
            let d = args.first().ok_or_else(|| RuntimeError::new("مفتاح_اصغر تتطلب مترابطة"))?;
            match d {
                Value::Dict(dict) => {
                    let borrow = dict.borrow();
                    let mut min_key: Option<Value> = None;
                    for (k, _) in borrow.iter() {
                        min_key = Some(match &min_key {
                            Some(mk) if mk.to_string_value() <= k.to_string_value() => mk.clone(),
                            _ => k.clone(),
                        });
                    }
                    Ok(min_key.unwrap_or(Value::Null))
                }
                _ => Err(RuntimeError::new_typed("استثناء_نوع", "مفتاح_اصغر تتطلب مترابطة")),
            }
        }
        "قيمة_اكبر" => {
            let d = args.first().ok_or_else(|| RuntimeError::new("قيمة_اكبر تتطلب مترابطة"))?;
            match d {
                Value::Dict(dict) => {
                    let borrow = dict.borrow();
                    let mut max_val: Option<Value> = None;
                    for (_, v) in borrow.iter() {
                        max_val = Some(match &max_val {
                            Some(mv) => {
                                if format!("{}", mv) >= format!("{}", v) { mv.clone() } else { v.clone() }
                            }
                            None => v.clone(),
                        });
                    }
                    Ok(max_val.unwrap_or(Value::Null))
                }
                _ => Err(RuntimeError::new_typed("استثناء_نوع", "قيمة_اكبر تتطلب مترابطة")),
            }
        }
        "قيمة_اصغر" => {
            let d = args.first().ok_or_else(|| RuntimeError::new("قيمة_اصغر تتطلب مترابطة"))?;
            match d {
                Value::Dict(dict) => {
                    let borrow = dict.borrow();
                    let mut min_val: Option<Value> = None;
                    for (_, v) in borrow.iter() {
                        min_val = Some(match &min_val {
                            Some(mv) => {
                                if format!("{}", mv) <= format!("{}", v) { mv.clone() } else { v.clone() }
                            }
                            None => v.clone(),
                        });
                    }
                    Ok(min_val.unwrap_or(Value::Null))
                }
                _ => Err(RuntimeError::new_typed("استثناء_نوع", "قيمة_اصغر تتطلب مترابطة")),
            }
        }
        "ادمج" => {
            let a = args.first().ok_or_else(|| RuntimeError::new("ادمج يتطلب مصفوفتين"))?;
            let b = args.get(1).ok_or_else(|| RuntimeError::new("ادمج يتطلب مصفوفتين"))?;
            match (a, b) {
                (Value::List(l1), Value::List(l2)) => {
                    let mut result: Vec<Value> = l1.borrow().clone();
                    result.extend(l2.borrow().iter().cloned());
                    Ok(Value::List(SharedList::new(result)))
                }
                _ => Err(RuntimeError::new_typed("استثناء_نوع", "ادمج تتطلب مصفوفتين")),
            }
        }
        "فرغ" => {
            Ok(Value::List(SharedList::new(vec![])))
        }
        "كرر" => {
            let item = args.first().ok_or_else(|| RuntimeError::new("كرر يتطلب عنصراً وعددأ"))?;
            let count = args.get(1).ok_or_else(|| RuntimeError::new("كرر يتطلب عدداً"))?;
            let n = match count {
                Value::Integer(n) => *n as usize,
                _ => return Err(RuntimeError::new_typed("استثناء_نوع", "كرر يتطلب عدداً صحيحاً")),
            };
            Ok(Value::List(SharedList::new(vec![item.clone(); n])))
        }
        "موجود" => {
            let item = args.first().ok_or_else(|| RuntimeError::new("موجود يتطلب عنصراً"))?;
            let list = args.get(1).ok_or_else(|| RuntimeError::new("موجود تتطلب مصفوفة"))?;
            match list {
                Value::List(l) => {
                    Ok(Value::Boolean(l.borrow().contains(item)))
                }
                _ => Err(RuntimeError::new_typed("استثناء_نوع", "موجود تتطلب مصفوفة")),
            }
        }

        "نمط_طابق" => {
            let pat = args.first().ok_or_else(|| RuntimeError::new("نمط_طابق يتطلب نمطاً"))?;
            let txt = args.get(1).ok_or_else(|| RuntimeError::new("نمط_طابق يتطلب نصاً"))?;
            match (pat, txt) {
                (Value::String(p), Value::String(t)) => {
                    let re = regex::Regex::new(p).map_err(|e| RuntimeError::new_typed("استثناء_نوع", format!("نمط غير صحيح: {}", e)))?;
                    Ok(Value::Boolean(re.is_match(t)))
                }
                _ => Err(RuntimeError::new_typed("استثناء_نوع", "نمط_طابق يتطلب نمطين")),
            }
        }
        "نمط_ابحث" => {
            let pat = args.first().ok_or_else(|| RuntimeError::new("نمط_ابحث يتطلب نمطاً"))?;
            let txt = args.get(1).ok_or_else(|| RuntimeError::new("نمط_ابحث يتطلب نصاً"))?;
            match (pat, txt) {
                (Value::String(p), Value::String(t)) => {
                    let re = regex::Regex::new(p).map_err(|e| RuntimeError::new_typed("استثناء_نوع", format!("نمط غير صحيح: {}", e)))?;
                    match re.find(t) {
                        Some(m) => {
                            let result = vec![
                                (Value::String(Rc::new("مطابقة".to_string())), Value::String(Rc::new(m.as_str().to_string()))),
                                (Value::String(Rc::new("بداية".to_string())), Value::Integer(m.start() as i64)),
                                (Value::String(Rc::new("نهاية".to_string())), Value::Integer(m.end() as i64)),
                            ];
                            Ok(Value::Dict(SharedDict::new(result)))
                        }
                        None => Ok(Value::Null),
                    }
                }
                _ => Err(RuntimeError::new_typed("استثناء_نوع", "نمط_ابحث يتطلب نمطين")),
            }
        }
        "نمط_كل_التطابقات" => {
            let pat = args.first().ok_or_else(|| RuntimeError::new("نمط_كل_التطابقات يتطلب نمطاً"))?;
            let txt = args.get(1).ok_or_else(|| RuntimeError::new("نمط_كل_التطابقات يتطلب نصاً"))?;
            match (pat, txt) {
                (Value::String(p), Value::String(t)) => {
                    let re = regex::Regex::new(p).map_err(|e| RuntimeError::new_typed("استثناء_نوع", format!("نمط غير صحيح: {}", e)))?;
                    let results: Vec<Value> = re.find_iter(t)
                        .map(|m| {
                            let d = vec![
                                (Value::String(Rc::new("مطابقة".to_string())), Value::String(Rc::new(m.as_str().to_string()))),
                                (Value::String(Rc::new("بداية".to_string())), Value::Integer(m.start() as i64)),
                                (Value::String(Rc::new("نهاية".to_string())), Value::Integer(m.end() as i64)),
                            ];
                            Value::Dict(SharedDict::new(d))
                        })
                        .collect();
                    Ok(Value::List(SharedList::new(results)))
                }
                _ => Err(RuntimeError::new_typed("استثناء_نوع", "نمط_كل_التطابقات يتطلب نمطين")),
            }
        }
        "نمط_استبدل" => {
            let pat = args.first().ok_or_else(|| RuntimeError::new("نمط_استبدل يتطلب نمطاً"))?;
            let txt = args.get(1).ok_or_else(|| RuntimeError::new("نمط_استبدل يتطلب نصاً"))?;
            let rep = args.get(2).ok_or_else(|| RuntimeError::new("نمط_استبدل يتطلب نصاً للاستبدال"))?;
            match (pat, txt, rep) {
                (Value::String(p), Value::String(t), Value::String(r)) => {
                    let re = regex::Regex::new(p).map_err(|e| RuntimeError::new_typed("استثناء_نوع", format!("نمط غير صحيح: {}", e)))?;
                    Ok(Value::String(Rc::new(re.replace_all(t, r.as_str()).to_string())))
                }
                _ => Err(RuntimeError::new_typed("استثناء_نوع", "نمط_استبدل يتطلب 3 نصوص")),
            }
        }
        "نمط_قسم" => {
            let pat = args.first().ok_or_else(|| RuntimeError::new("نمط_قسم يتطلب نمطاً"))?;
            let txt = args.get(1).ok_or_else(|| RuntimeError::new("نمط_قسم يتطلب نصاً"))?;
            match (pat, txt) {
                (Value::String(p), Value::String(t)) => {
                    let re = regex::Regex::new(p).map_err(|e| RuntimeError::new_typed("استثناء_نوع", format!("نمط غير صحيح: {}", e)))?;
                    let parts: Vec<Value> = re.split(t)
                        .map(|s| Value::String(Rc::new(s.to_string())))
                        .collect();
                    Ok(Value::List(SharedList::new(parts)))
                }
                _ => Err(RuntimeError::new_typed("استثناء_نوع", "نمط_قسم يتطلب نمطين")),
            }
        }
        "نمط_جميع" => {
            let pat = args.first().ok_or_else(|| RuntimeError::new("نمط_جميع يتطلب نمطاً"))?;
            let txt = args.get(1).ok_or_else(|| RuntimeError::new("نمط_جميع يتطلب نصاً"))?;
            match (pat, txt) {
                (Value::String(p), Value::String(t)) => {
                    let re = regex::Regex::new(p).map_err(|e| RuntimeError::new_typed("استثناء_نوع", format!("نمط غير صحيح: {}", e)))?;
                    let captures: Vec<Value> = re.captures_iter(t)
                        .filter_map(|cap| {
                            cap.get(1).map(|m| Value::String(Rc::new(m.as_str().to_string())))
                        })
                        .collect();
                    Ok(Value::List(SharedList::new(captures)))
                }
                _ => Err(RuntimeError::new_typed("استثناء_نوع", "نمط_جميع يتطلب نمطين")),
            }
        }
        "خاصية" => {
            let obj = args.first().ok_or_else(|| RuntimeError::new("خاصية تتطلب كائناً"))?;
            let name = args.get(1).ok_or_else(|| RuntimeError::new("خاصية تتطلب اسم خاصية"))?;
            let name_str = match name {
                Value::String(s) => s.to_string(),
                _ => return Err(RuntimeError::new_typed("استثناء_نوع", "اسم الخاصية يجب ان يكون نصاً")),
            };
            match obj.get_attribute(&name_str) {
                Some(val) => Ok(val),
                None => Err(RuntimeError::new_typed("استثناء_اسم", format!("خاصية غير موجودة: {}", name_str))),
            }
        }
        "تعيين_خاصية" => {
            let obj = args.first().ok_or_else(|| RuntimeError::new("تعيين_خاصية تتطلب كائناً"))?;
            let name = args.get(1).ok_or_else(|| RuntimeError::new("تعيين_خاصية تتطلب اسم خاصية"))?;
            let value = args.get(2).ok_or_else(|| RuntimeError::new("تعيين_خاصية تتطلب قيمة"))?;
            let name_str = match name {
                Value::String(s) => s.to_string(),
                _ => return Err(RuntimeError::new_typed("استثناء_نوع", "اسم الخاصية يجب ان يكون نصاً")),
            };
            match obj {
                Value::Instance(rc) => {
                    rc.set_field(name_str, value.clone());
                    Ok(Value::Null)
                }
                _ => Err(RuntimeError::new_typed("استثناء_نوع", "تعيين_خاصية تتطلب كائناً")),
            }
        }
        "هل_خاصية" => {
            let obj = args.first().ok_or_else(|| RuntimeError::new("هل_خاصية تتطلب كائناً"))?;
            let name = args.get(1).ok_or_else(|| RuntimeError::new("هل_خاصية تتطلب اسم خاصية"))?;
            let name_str = match name {
                Value::String(s) => s.to_string(),
                _ => return Err(RuntimeError::new_typed("استثناء_نوع", "اسم الخاصية يجب ان يكون نصاً")),
            };
            Ok(Value::Boolean(obj.get_attribute(&name_str).is_some()))
        }

        // === شبكة (Network/HTTP) module ===
        "شبكة_جلب" | "شبكة_جلب_نص" => {
            let url = args.first()
                .ok_or_else(|| RuntimeError::new("شبكة.جلب يتطلب رابطاً"))?
                .to_string_value();
            match ureq::get(&url).call() {
                Ok(resp) => {
                    let status = resp.status();
                    let mut body = String::new();
                    resp.into_reader().read_to_string(&mut body).map_err(|e| {
                        RuntimeError::new(format!("فشل قراءة الاستجابة: {}", e))
                    })?;
                    if name == "شبكة_جلب" {
                        let dict_items = vec![
                            (Value::String(Rc::new("الحالة".to_string())), Value::Integer(status as i64)),
                            (Value::String(Rc::new("المحتوى".to_string())), Value::String(body.into())),
                        ];
                        Ok(Value::Dict(SharedDict::new(dict_items)))
                    } else {
                        Ok(Value::String(body.into()))
                    }
                }
                Err(e) => Err(RuntimeError::new(format!("فشل الطلب: {}", e))),
            }
        }
        "شبكة_جلب_كائن" => {
            let url = args.first()
                .ok_or_else(|| RuntimeError::new("شبكة.جلب_كائن يتطلب رابطاً"))?
                .to_string_value();
            match ureq::get(&url).call() {
                Ok(resp) => {
                    let mut body = String::new();
                    resp.into_reader().read_to_string(&mut body).map_err(|e| {
                        RuntimeError::new(format!("فشل قراءة الاستجابة: {}", e))
                    })?;
                    let json_val: serde_json::Value = serde_json::from_str(&body)
                        .map_err(|e| RuntimeError::new(format!("فشل تحليل JSON: {}", e)))?;
                    Ok(json_value_to_arabi(&json_val))
                }
                Err(e) => Err(RuntimeError::new(format!("فشل الطلب: {}", e))),
            }
        }
        "شبكة_ارسل" => {
            let url = args.first()
                .ok_or_else(|| RuntimeError::new("شبكة.ارسل يتطلب رابطاً"))?
                .to_string_value();
            let body = args.get(1)
                .map(|v| v.to_string_value())
                .unwrap_or_default();
            match ureq::post(&url).send_string(&body) {
                Ok(resp) => {
                    let status = resp.status();
                    let mut resp_body = String::new();
                    resp.into_reader().read_to_string(&mut resp_body).map_err(|e| {
                        RuntimeError::new(format!("فشل قراءة الاستجابة: {}", e))
                    })?;
                    let dict_items = vec![
                        (Value::String(Rc::new("الحالة".to_string())), Value::Integer(status as i64)),
                        (Value::String(Rc::new("المحتوى".to_string())), Value::String(resp_body.into())),
                    ];
                    Ok(Value::Dict(SharedDict::new(dict_items)))
                }
                Err(e) => Err(RuntimeError::new(format!("فشل الارسال: {}", e))),
            }
        }
        "شبكة_ارسل_كائن" => {
            let url = args.first()
                .ok_or_else(|| RuntimeError::new("شبكة.ارسل_كائن يتطلب رابطاً"))?
                .to_string_value();
            let json_val = args.get(1)
                .ok_or_else(|| RuntimeError::new("شبكة.ارسل_كائن يتطلب كائناً"))?;
            let json_str = arabi_value_to_json(json_val)
                .map_err(|e| RuntimeError::new(format!("فشل تحويل القيمة لـ JSON: {}", e)))?;
            let body = serde_json::to_string(&json_str)
                .map_err(|e| RuntimeError::new(format!("فشل تحويل JSON لنص: {}", e)))?;
            match ureq::post(&url)
                .set("Content-Type", "application/json")
                .send_string(&body)
            {
                Ok(resp) => {
                    let status = resp.status();
                    let mut resp_body = String::new();
                    resp.into_reader().read_to_string(&mut resp_body).map_err(|e| {
                        RuntimeError::new(format!("فشل قراءة الاستجابة: {}", e))
                    })?;
                    let resp_json: serde_json::Value = serde_json::from_str(&resp_body)
                        .unwrap_or(serde_json::Value::String(resp_body.clone()));
                    let dict_items = vec![
                        (Value::String(Rc::new("الحالة".to_string())), Value::Integer(status as i64)),
                        (Value::String(Rc::new("المحتوى".to_string())), json_value_to_arabi(&resp_json)),
                    ];
                    Ok(Value::Dict(SharedDict::new(dict_items)))
                }
                Err(e) => Err(RuntimeError::new(format!("فشل الارسال: {}", e))),
            }
        }
        "شبكة_طلب" => {
            let method = args.first()
                .ok_or_else(|| RuntimeError::new("شبكة.طلب يتطلب طريقة"))?
                .to_string_value().to_uppercase();
            let url = args.get(1)
                .ok_or_else(|| RuntimeError::new("شبكة.طلب يتطلب رابطاً"))?
                .to_string_value();
            let body = args.get(2)
                .map(|v| v.to_string_value())
                .unwrap_or_default();
            let req = match method.as_str() {
                "GET" => ureq::get(&url),
                "POST" => ureq::post(&url),
                "PUT" => ureq::put(&url),
                "DELETE" => ureq::delete(&url),
                "PATCH" => ureq::patch(&url),
                _ => return Err(RuntimeError::new(format!("طريقة HTTP غير معروفة: {}", method))),
            };
            match req.send_string(&body) {
                Ok(resp) => {
                    let status = resp.status();
                    let mut resp_body = String::new();
                    resp.into_reader().read_to_string(&mut resp_body).map_err(|e| {
                        RuntimeError::new(format!("فشل قراءة الاستجابة: {}", e))
                    })?;
                    let dict_items = vec![
                        (Value::String(Rc::new("الحالة".to_string())), Value::Integer(status as i64)),
                        (Value::String(Rc::new("المحتوى".to_string())), Value::String(resp_body.into())),
                    ];
                    Ok(Value::Dict(SharedDict::new(dict_items)))
                }
                Err(e) => Err(RuntimeError::new(format!("فشل الطلب: {}", e))),
            }
        }

        // === عمليات (Subprocess) module ===
        "عمليات_نفاذ" => {
            let cmd = args.first()
                .ok_or_else(|| RuntimeError::new("عمليات.نفاذ يتطلب امراً"))?
                .to_string_value();
            let output = if cfg!(windows) {
                std::process::Command::new("cmd")
                    .arg("/C")
                    .arg(&cmd)
                    .output()
            } else {
                std::process::Command::new("sh")
                    .arg("-c")
                    .arg(&cmd)
                    .output()
            }.map_err(|e| RuntimeError::new(format!("فشل تنفيذ الأمر '{}': {}", cmd, e)))?;
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            let code = output.status.code().unwrap_or(-1);
            let dict_items = vec![
                (Value::String(Rc::new("المخرجات".to_string())), Value::String(stdout.into())),
                (Value::String(Rc::new("الاخطاء".to_string())), Value::String(stderr.into())),
                (Value::String(Rc::new("الحالة".to_string())), Value::Integer(code as i64)),
            ];
            Ok(Value::Dict(SharedDict::new(dict_items)))
        }
        "عمليات_نفاذ_مع" => {
            let cmd = args.first()
                .ok_or_else(|| RuntimeError::new("عمليات.نفاذ_مع يتطلب امراً"))?
                .to_string_value();
            let input = args.get(1)
                .map(|v| v.to_string_value())
                .unwrap_or_default();
            use std::io::Write;
            let mut child = if cfg!(windows) {
                std::process::Command::new("cmd")
                    .arg("/C")
                    .arg(&cmd)
                    .stdin(std::process::Stdio::piped())
                    .stdout(std::process::Stdio::piped())
                    .stderr(std::process::Stdio::piped())
                    .spawn()
            } else {
                std::process::Command::new("sh")
                    .arg("-c")
                    .arg(&cmd)
                    .stdin(std::process::Stdio::piped())
                    .stdout(std::process::Stdio::piped())
                    .stderr(std::process::Stdio::piped())
                    .spawn()
            }.map_err(|e| RuntimeError::new(format!("فشل تنفيذ الأمر '{}': {}", cmd, e)))?;
            if let Some(mut stdin) = child.stdin.take() {
                stdin.write_all(input.as_bytes()).map_err(|e| {
                    RuntimeError::new(format!("فشل كتابة المدخلات: {}", e))
                })?;
            }
            let output = child.wait_with_output()
                .map_err(|e| RuntimeError::new(format!("فشل انتظار الأمر: {}", e)))?;
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            let code = output.status.code().unwrap_or(-1);
            let dict_items = vec![
                (Value::String(Rc::new("المخرجات".to_string())), Value::String(stdout.into())),
                (Value::String(Rc::new("الاخطاء".to_string())), Value::String(stderr.into())),
                (Value::String(Rc::new("الحالة".to_string())), Value::Integer(code as i64)),
            ];
            Ok(Value::Dict(SharedDict::new(dict_items)))
        }
        "عمليات_نفاذ_قائمة" => {
            let cmds_val = args.first()
                .ok_or_else(|| RuntimeError::new("عمليات.نفاذ_قائمة تتطلب قائمة امروات"))?;
            let cmds = match cmds_val {
                Value::List(l) => l,
                _ => return Err(RuntimeError::new_typed("استثناء_نوع", "المعامل يجب ان يكون قائمة")),
            };
            let mut results = Vec::new();
            let borrowed = cmds.borrow();
            for cmd_val in borrowed.iter() {
                let cmd = cmd_val.to_string_value();
                let output = if cfg!(windows) {
                    std::process::Command::new("cmd")
                        .arg("/C")
                        .arg(&cmd)
                        .output()
                } else {
                    std::process::Command::new("sh")
                        .arg("-c")
                        .arg(&cmd)
                        .output()
                }.map_err(|e| RuntimeError::new(format!("فشل تنفيذ الأمر '{}': {}", cmd, e)))?;
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                let code = output.status.code().unwrap_or(-1);
                let dict_items = vec![
                    (Value::String(Rc::new("المخرجات".to_string())), Value::String(stdout.into())),
                    (Value::String(Rc::new("الاخطاء".to_string())), Value::String(stderr.into())),
                    (Value::String(Rc::new("الحالة".to_string())), Value::Integer(code as i64)),
                ];
                results.push(Value::Dict(SharedDict::new(dict_items)));
            }
            Ok(Value::List(SharedList::new(results)))
        }

        _ => Err(RuntimeError::new_typed("استثناء_اسم", format!("الدالة النظامية '{}' غير موجودة", name))),
    }
}

fn get_float_arg(args: &[Value], index: usize) -> Result<f64, RuntimeError> {
    let arg = args.get(index)
        .ok_or_else(|| RuntimeError::new(format!("المعامل مفقود في الموقع {}", index + 1)))?;
    match arg {
        Value::Integer(n) => Ok(*n as f64),
        Value::Float(f) => Ok(*f),
        _ => Err(RuntimeError::new("المعامل يجب ان يكون عدداً")),
    }
}

fn get_optional_int_arg(args: &[Value], index: usize) -> Option<i64> {
    args.get(index).and_then(|v| match v {
        Value::Integer(n) => Some(*n),
        _ => None,
    })
}

fn json_value_to_arabi(val: &serde_json::Value) -> Value {
    match val {
        serde_json::Value::Null => Value::Null,
        serde_json::Value::Bool(b) => Value::Boolean(*b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Value::Integer(i)
            } else if let Some(f) = n.as_f64() {
                Value::Float(f)
            } else {
                Value::Null
            }
        }
        serde_json::Value::String(s) => Value::String(Rc::new(s.clone())),
        serde_json::Value::Array(arr) => {
            let items: Vec<Value> = arr.iter().map(json_value_to_arabi).collect();
            Value::List(SharedList::new(items))
        }
        serde_json::Value::Object(map) => {
            let items: Vec<(Value, Value)> = map.iter()
                .map(|(k, v)| (Value::String(k.clone().into()), json_value_to_arabi(v)))
                .collect();
            Value::Dict(SharedDict::new(items))
        }
    }
}

fn arabi_value_to_json(val: &Value) -> Result<serde_json::Value, RuntimeError> {
    match val {
        Value::Null => Ok(serde_json::Value::Null),
        Value::Boolean(b) => Ok(serde_json::Value::Bool(*b)),
        Value::Integer(i) => Ok(serde_json::Value::Number((*i).into())),
        Value::Float(f) => {
            if let Some(n) = serde_json::Number::from_f64(*f) {
                Ok(serde_json::Value::Number(n))
            } else {
                Ok(serde_json::Value::Null)
            }
        }
        Value::String(s) => Ok(serde_json::Value::String(s.to_string())),
        Value::List(list) => {
            let items: Result<Vec<serde_json::Value>, _> = list.borrow().iter()
                .map(arabi_value_to_json)
                .collect();
            Ok(serde_json::Value::Array(items?))
        }
        Value::Dict(dict) => {
            let mut map = serde_json::Map::new();
            for (k, v) in dict.borrow().iter() {
                let key = k.to_string_value();
                map.insert(key, arabi_value_to_json(v)?);
            }
            Ok(serde_json::Value::Object(map))
        }
        _ => Err(RuntimeError::new(format!("النوع '{}' لا يمكن تحويله لـ JSON", val.type_name()))),
    }
}

fn json_to_arabi(val: serde_json::Value) -> Value {
    match val {
        serde_json::Value::Null => Value::Null,
        serde_json::Value::Bool(b) => Value::Boolean(b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Value::Integer(i)
            } else {
                Value::Float(n.as_f64().unwrap_or(0.0))
            }
        }
        serde_json::Value::String(s) => Value::String(Rc::new(s)),
        serde_json::Value::Array(arr) => {
            let items: Vec<Value> = arr.into_iter().map(json_to_arabi).collect();
            Value::List(SharedList::new(items))
        }
        serde_json::Value::Object(map) => {
            let items: Vec<(Value, Value)> = map.into_iter()
                .map(|(k, v)| (Value::String(Rc::new(k)), json_to_arabi(v)))
                .collect();
            Value::Dict(SharedDict::new(items))
        }
    }
}
