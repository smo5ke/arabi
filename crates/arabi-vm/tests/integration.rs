use arabi_lexer::Lexer;
use arabi_parser::Parser;
use arabi_compiler::Compiler;
use arabi_vm::VM;
use arabi_vm::{Value, SharedList};
use std::collections::HashMap;
use std::rc::Rc;

#[derive(Debug)]
struct RunResult {
    globals: HashMap<String, Value>,
}

fn run_arabi(source: &str) -> Result<RunResult, String> {
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().map_err(|e| format!("Lexer error: {:?}", e))?;
    let mut parser = Parser::new(tokens);
    let ast = parser.parse().map_err(|e| format!("Parser error: {:?}", e))?;
    let mut compiler = Compiler::new();
    let mut bytecode = compiler.compile(&ast).map_err(|e| format!("Compiler error: {:?}", e))?;
    let mut vm = VM::new();
    vm.execute(&mut bytecode).map_err(|e| format!("VM error: {:?}", e))?;
    Ok(RunResult { globals: vm.globals })
}

fn run_get(source: &str, var: &str) -> Value {
    let r = run_arabi(source).unwrap();
    r.globals.get(var).cloned().unwrap_or(Value::Null)
}

// ============================================================================
// 1. Variables & Assignment
// ============================================================================

#[test]
fn test_01_variable_assignment() {
    assert_eq!(run_get("س = 42", "س"), Value::Integer(42));
}

#[test]
fn test_02_walrus_operator() {
    assert_eq!(run_get("س = 0\nس = (م := 42)", "م"), Value::Integer(42));
    assert_eq!(run_get("س = 0\nس = (م := 42)", "س"), Value::Integer(42));
}

#[test]
fn test_03_multi_assign() {
    let r = run_arabi("ا، ب، ج = 1، 2، 3").unwrap();
    assert_eq!(r.globals.get("ا"), Some(&Value::Integer(1)));
    assert_eq!(r.globals.get("ب"), Some(&Value::Integer(2)));
    assert_eq!(r.globals.get("ج"), Some(&Value::Integer(3)));
}

#[test]
fn test_04_star_unpack_end() {
    let r = run_arabi("ا، *ب = [1، 2، 3، 4، 5]").unwrap();
    assert_eq!(r.globals.get("ا"), Some(&Value::Integer(1)));
    match r.globals.get("ب").unwrap() {
        Value::List(items) => assert_eq!(items.borrow().len(), 4),
        _ => panic!("Expected List"),
    }
}

#[test]
fn test_05_star_unpack_start() {
    let r = run_arabi("*ا، ب = [10، 20، 30]").unwrap();
    match r.globals.get("ا").unwrap() {
        Value::List(items) => {
            let b = items.borrow();
            assert_eq!(b.len(), 2);
            assert_eq!(b[0], Value::Integer(10));
        }
        _ => panic!("Expected List"),
    }
    assert_eq!(r.globals.get("ب"), Some(&Value::Integer(30)));
}

#[test]
fn test_06_star_unpack_middle() {
    let r = run_arabi("ا، *م، ب = [1، 2، 3، 4، 5]").unwrap();
    assert_eq!(r.globals.get("ا"), Some(&Value::Integer(1)));
    match r.globals.get("م").unwrap() {
        Value::List(items) => {
            let b = items.borrow();
            assert_eq!(b.len(), 3);
        }
        _ => panic!("Expected List"),
    }
    assert_eq!(r.globals.get("ب"), Some(&Value::Integer(5)));
}

// ============================================================================
// 2. Arithmetic
// ============================================================================

#[test]
fn test_07_arithmetic() {
    assert_eq!(run_get("ن = 2 + 3", "ن"), Value::Integer(5));
    assert_eq!(run_get("ن = 10 - 3", "ن"), Value::Integer(7));
    assert_eq!(run_get("ن = 4 * 5", "ن"), Value::Integer(20));
}

#[test]
fn test_08_floor_div() {
    assert_eq!(run_get("ن = 7 \\ 2", "ن"), Value::Integer(3));
}

#[test]
fn test_09_power() {
    assert_eq!(run_get("ن = 2 ^ 10", "ن"), Value::Integer(1024));
}

#[test]
fn test_10_augmented_power() {
    assert_eq!(run_get("س = 2\nس **= 10", "س"), Value::Integer(1024));
}

// ============================================================================
// 3. Comparisons
// ============================================================================

#[test]
fn test_11_comparison() {
    assert_eq!(run_get("ن = 5 > 3", "ن"), Value::Boolean(true));
    assert_eq!(run_get("ن = 3 < 5", "ن"), Value::Boolean(true));
    assert_eq!(run_get("ن = 5 == 5", "ن"), Value::Boolean(true));
    assert_eq!(run_get("ن = 5 != 3", "ن"), Value::Boolean(true));
}

#[test]
fn test_12_is_is_not() {
    assert_eq!(run_get("ن = عدم يساوي عدم", "ن"), Value::Boolean(true));
    assert_eq!(run_get("ن = 5 يساوي ليس 3", "ن"), Value::Boolean(true));
}

#[test]
fn test_13_chained_comparison() {
    assert_eq!(run_get("ن = 10 < 15 < 20", "ن"), Value::Boolean(true));
    assert_eq!(run_get("ن = 1 < 2 > 0", "ن"), Value::Boolean(true));
    assert_eq!(run_get("ن = 10 > 5 > 3 > 1", "ن"), Value::Boolean(true));
}

// ============================================================================
// 4. Lists
// ============================================================================

#[test]
fn test_14_list_literal() {
    match run_get("ن = [1، 2، 3]", "ن") {
        Value::List(items) => assert_eq!(items.borrow().len(), 3),
        _ => panic!("Expected List"),
    }
}

#[test]
fn test_15_list_comprehension() {
    match run_get("ن = [س ^ 2 لكل س في مدى(5)]", "ن") {
        Value::List(items) => {
            let b = items.borrow();
            assert_eq!(b.len(), 5);
            assert_eq!(b[0], Value::Integer(0));
            assert_eq!(b[4], Value::Integer(16));
        }
        _ => panic!("Expected List"),
    }
}

#[test]
fn test_16_list_comprehension_condition() {
    match run_get("ن = [س لكل س في مدى(8) اذا س % 2 == 0]", "ن") {
        Value::List(items) => assert_eq!(items.borrow().len(), 4),
        _ => panic!("Expected List"),
    }
}

// ============================================================================
// 5. Dicts
// ============================================================================

#[test]
fn test_17_dict_literal() {
    match run_get(r#"ن = {"ا": 1، "ب": 2}"#, "ن") {
        Value::Dict(_) => {}
        _ => panic!("Expected Dict"),
    }
}

#[test]
fn test_18_dict_comprehension() {
    match run_get("{س: س ^ 2 لكل س في [1، 2، 3]}", "س") {
        _ => {}
    }
}

// ============================================================================
// 6. Sets
// ============================================================================

#[test]
fn test_19_set_literal() {
    match run_get("ن = مميزة([1، 2، 3، 1، 2])", "ن") {
        Value::Set(_) => {}
        _ => panic!("Expected Set"),
    }
}

// ============================================================================
// 7. Functions
// ============================================================================

#[test]
fn test_20_function_def_call() {
    let r = run_arabi("دالة مضاعف(س):\n    ارجع س * 2\nن = مضاعف(21)").unwrap();
    assert_eq!(r.globals.get("ن"), Some(&Value::Integer(42)));
}

#[test]
fn test_21_lambda() {
    let r = run_arabi("اضف = خطية ا، ب: ا + ب\nن = اضف(3، 4)").unwrap();
    assert_eq!(r.globals.get("ن"), Some(&Value::Integer(7)));
}

#[test]
fn test_22_varargs() {
    let r = run_arabi(
        "دالة مجموع(*ارقام):\n    ن = 0\n    لكل ع في ارقام:\n        ن += ع\n    ارجع ن\nن = مجموع(1، 2، 3، 4، 5)"
    ).unwrap();
    assert_eq!(r.globals.get("ن"), Some(&Value::Integer(15)));
}

// ============================================================================
// 8. Loops
// ============================================================================

#[test]
fn test_23_while_loop() {
    assert_eq!(run_get("س = 0\nبينما س < 5:\n    س += 1", "س"), Value::Integer(5));
}

#[test]
fn test_24_for_range() {
    assert_eq!(run_get("ن = 0\nلكل ع في مدى(1، 6):\n    ن = ن + ع", "ن"), Value::Integer(15));
}

#[test]
fn test_25_for_tuple_unpack() {
    assert_eq!(
        run_get("ن = 0\nلكل (ا، ب) في [[1، 2]، [3، 4]، [5، 6]]:\n    ن += ا + ب", "ن"),
        Value::Integer(21)
    );
}

// ============================================================================
// 9. Conditionals
// ============================================================================

#[test]
fn test_26_if_else() {
    assert_eq!(
        run_get("س = 10\nاذا س > 5:\n    س = 100\nوالا:\n    س = 0", "س"),
        Value::Integer(100)
    );
}

#[test]
fn test_27_if_elif_else() {
    assert_eq!(
        run_get("س = 15\nاذا س > 10:\n    س = 1\nوالا:\n    س = 3", "س"),
        Value::Integer(1)
    );
}

#[test]
fn test_28_ternary() {
    let r = run_arabi(r#"س = 10
النتيجة = "كبير" اذا س > 5 والا "صغير""#).unwrap();
    assert_eq!(r.globals.get("النتيجة"), Some(&Value::String(Rc::new("كبير".to_string()))));
}

// ============================================================================
// 11. Classes
// ============================================================================

#[test]
fn test_31_class_basic() {
    let r = run_arabi(r#"صنف نقطة:
    دالة __تهيئة__(هذا، س، ص):
        هذا.س = س
        هذا.ص = ص
    دالة مسافة(هذا):
        ارجع هذا.س + هذا.ص
ن = نقطة(3، 4)
ن2 = ن.مسافة()"#).unwrap();
    assert_eq!(r.globals.get("ن2"), Some(&Value::Integer(7)));
}

#[test]
fn test_32_class_inheritance() {
    let r = run_arabi(r#"صنف حيوان:
    دالة __تهيئة__(هذا، اسم):
        هذا.اسم = اسم
صنف كلب(حيوان):
    دالة نباح(هذا):
        ارجع "نبح! " + هذا.اسم
ك = كلب("بودي")
ن2 = ك.نباح()"#).unwrap();
    assert_eq!(r.globals.get("ن2"), Some(&Value::String(Rc::new("نبح! بودي".to_string()))));
}

// ============================================================================
// 12. Error Handling
// ============================================================================

#[test]
fn test_33_try_except() {
    assert_eq!(
        run_get("حاول:\n    س = 1 / 0\nخلل:\n    س = 99", "س"),
        Value::Integer(99)
    );
}

#[test]
fn test_34_finally() {
    assert_eq!(
        run_get("ن = 0\nحاول:\n    ن = 1\nنهاية:\n    ن = 2", "ن"),
        Value::Integer(2)
    );
}

// ============================================================================
// 13. Generators
// ============================================================================

#[test]
fn test_35_generator() {
    let r = run_arabi(
        "دالة عداد(حد):\n    س = 0\n    بينما س < حد:\n        سلم س\n        س += 1\nالقائمة = []\nلكل ع في عداد(5):\n    القائمة.اضف(ع)"
    ).unwrap();
    match r.globals.get("القائمة").unwrap() {
        Value::List(items) => {
            let b = items.borrow();
            assert_eq!(b.len(), 5);
            assert_eq!(b[0], Value::Integer(0));
            assert_eq!(b[4], Value::Integer(4));
        }
        _ => panic!("Expected List"),
    }
}

// ============================================================================
// 14. Strings
// ============================================================================

#[test]
fn test_36_string_length() {
    assert_eq!(run_get(r#"ن = طول("مرحبا")"#, "ن"), Value::Integer(5));
}

#[test]
fn test_37_string_methods() {
    assert_eq!(
        run_get(r#"ن = "مرحبا بالعالم".يبدا("مرحبا")"#, "ن"),
        Value::Boolean(true)
    );
}

#[test]
fn test_38_fstring() {
    assert_eq!(
        run_get(r#"س = 42
ن = م"{س} نجح!""#, "ن"),
        Value::String(Rc::new("42 نجح!".to_string()))
    );
}

#[test]
fn test_38b_fstring_f_prefix() {
    assert_eq!(
        run_get(r#"س = 10
ن = ف"{س} × 2 = {س * 2}""#, "ن"),
        Value::String(Rc::new("10 × 2 = 20".to_string()))
    );
}

#[test]
fn test_38c_fstring_single_quotes() {
    assert_eq!(
        run_get(r#"ق = 99
ن = م'قيمة = {ق}'"#, "ن"),
        Value::String(Rc::new("قيمة = 99".to_string()))
    );
}

#[test]
fn test_38d_fstring_f_single_quotes() {
    assert_eq!(
        run_get(r#"ق = 5
ن = ف'القيمة = {ق}'"#, "ن"),
        Value::String(Rc::new("القيمة = 5".to_string()))
    );
}

#[test]
fn test_38e_fstring_multi_expr() {
    assert_eq!(
        run_get(r#"اسم = "أحمد"
عمر = 25
ن = م"{اسم} عمره {عمر} سنة""#, "ن"),
        Value::String(Rc::new("أحمد عمره 25 سنة".to_string()))
    );
}

// ============================================================================
// 15. Ranges & Slicing
// ============================================================================

#[test]
fn test_39_range() {
    match run_get("ن = مدى(5)", "ن") {
        Value::Range(d) => {
            assert_eq!(d.start, 0);
            assert_eq!(d.end, 5);
            assert_eq!(d.step, 1);
        }
        _ => panic!("Expected Range"),
    }
}

#[test]
fn test_40_list_slice() {
    match run_get("ق = [0، 1، 2، 3، 4]\nن = ق[1:4]", "ن") {
        Value::List(items) => assert_eq!(items.borrow().len(), 3),
        _ => panic!("Expected List"),
    }
}

// ============================================================================
// 16. Builtins
// ============================================================================

#[test]
fn test_41_all_builtin() {
    assert_eq!(
        run_get("ن = تحقق_اي([صح، صح، صح])", "ن"),
        Value::Boolean(true)
    );
    assert_eq!(
        run_get("ن = تحقق_اي([صح، خطا، صح])", "ن"),
        Value::Boolean(false)
    );
}

#[test]
fn test_42_enumerate() {
    match run_get("ن = تتبع([10، 20، 30])", "ن") {
        Value::List(items) => assert_eq!(items.borrow().len(), 3),
        _ => panic!("Expected List"),
    }
}

#[test]
fn test_43_map() {
    match run_get("ن = ضغط(خطية س: س * 2، [1، 2، 3])", "ن") {
        Value::List(items) => assert_eq!(items.borrow().len(), 3),
        _ => panic!("Expected List"),
    }
}

#[test]
fn test_44_filter() {
    match run_get("ن = تصفية(خطية س: س % 2 == 0، [1، 2، 3، 4، 5، 6])", "ن") {
        Value::List(items) => assert_eq!(items.borrow().len(), 3),
        _ => panic!("Expected List"),
    }
}

// ============================================================================
// 17. Nested Functions / Closures
// ============================================================================

#[test]
fn test_45_nested_function() {
    let r = run_arabi("دالة خارجية():\n    دالة داخلية():\n        ارجع 42\n    ارجع داخلية\nن = خارجية()()").unwrap();
    assert_eq!(r.globals.get("ن"), Some(&Value::Integer(42)));
}

#[test]
fn test_46_closure() {
    let r = run_arabi("دالة صانع():\n    ع = 0\n    دالة عداد():\n        محلي ع\n        ع += 1\n        ارجع ع\n    ارجع عداد\nع = صانع()\nع()\nع()\nالناتج = ع()\n").unwrap();
    assert_eq!(r.globals.get("الناتج"), Some(&Value::Integer(3)));
}

// ============================================================================
// 18. List Methods
// ============================================================================

#[test]
fn test_47_list_sort() {
    let r = run_arabi("ق = [3، 1، 4، 1، 5]\nق.رتب()").unwrap();
    match r.globals.get("ق").unwrap() {
        Value::List(items) => {
            let b = items.borrow();
            assert_eq!(b[0], Value::Integer(1));
        }
        _ => panic!("Expected List"),
    }
}

#[test]
fn test_48_list_append() {
    let r = run_arabi("ق = [1، 2، 3]\nق.اضف(4)").unwrap();
    match r.globals.get("ق").unwrap() {
        Value::List(items) => assert_eq!(items.borrow().len(), 4),
        _ => panic!("Expected List"),
    }
}

// ============================================================================
// 19. Dict Methods
// ============================================================================

#[test]
fn test_49_dict_length() {
    assert_eq!(
        run_get(r#"ق = {"ا": 1، "ب": 2، "ج": 3}
ن = ق.طول"#, "ن"),
        Value::Integer(3)
    );
}

// ============================================================================
// 20. Yield From
// ============================================================================

#[test]
fn test_50_yield_from() {
    let r = run_arabi("اد = [10، 20، 30]\nدالة مولّد(مصدر):\n    سلم_من(مصدر)\nالنتيجة = []\nلكل ع في مولّد(اد):\n    النتيجة.اضف(ع)").unwrap();
    match r.globals.get("النتيجة").unwrap() {
        Value::List(items) => {
            let b = items.borrow();
            assert_eq!(b.len(), 3);
            assert_eq!(b[0], Value::Integer(10));
        }
        _ => panic!("Expected List"),
    }
}

// ============================================================================
// 21. Deep Equality (Tuple/List/Dict/Set)
// ============================================================================

#[test]
fn test_51_tuple_equality() {
    assert_eq!(run_get("نتيجة = (1، 2) == (1، 2)", "نتيجة"), Value::Boolean(true));
    assert_eq!(run_get("نتيجة2 = (1، 2) == (1، 3)", "نتيجة2"), Value::Boolean(false));
}

#[test]
fn test_52_list_equality() {
    assert_eq!(run_get("نتيجة = [1، 2] == [1، 2]", "نتيجة"), Value::Boolean(true));
    assert_eq!(run_get("نتيجة2 = [1، 2] == [1، 3]", "نتيجة2"), Value::Boolean(false));
}

#[test]
fn test_53_dict_equality() {
    assert_eq!(run_get("نتيجة = {\"ا\": 1} == {\"ا\": 1}", "نتيجة"), Value::Boolean(true));
    assert_eq!(run_get("نتيجة2 = {\"ا\": 1} == {\"ب\": 1}", "نتيجة2"), Value::Boolean(false));
}

// ============================================================================
// 22. Edge Cases
// ============================================================================

#[test]
fn test_54_recursion() {
    assert_eq!(
        run_get("دالة فايتوريال(ن):\n    اذا ن <= 1:\n        ارجع 1\n    ارجع ن * فايتوريال(ن - 1)\nنتيجة = فايتوريال(5)", "نتيجة"),
        Value::Integer(120)
    );
}

#[test]
fn test_55_nested_closures_3_levels() {
    let r = run_arabi("دالة ص1():\n    ع = 1\n    دالة ص2():\n        ع = ع + 10\n        دالة ص3():\n            ع = ع + 100\n            ارجع ع\n        ارجع ص3()\n    ارجع ص2()\nنتيجة = ص1()").unwrap();
    assert_eq!(r.globals.get("نتيجة"), Some(&Value::Integer(111)));
}

#[test]
fn test_56_string_operations() {
    assert_eq!(run_get("نتيجة = \"مرحبا\" + \" \" + \"عالم\"", "نتيجة"), Value::String(Rc::new("مرحبا عالم".to_string())));
    assert_eq!(run_get("نتيجة2 = \"ا\" * 3", "نتيجة2"), Value::String(Rc::new("ااا".to_string())));
}

#[test]
fn test_57_chained_comparison() {
    assert_eq!(run_get("س = 5\nنتيجة = 1 < س < 10", "نتيجة"), Value::Boolean(true));
    assert_eq!(run_get("س = 5\nنتيجة2 = 10 < س < 20", "نتيجة2"), Value::Boolean(false));
}

#[test]
fn test_58_nested_data_structures() {
    let r = run_arabi("ق = [[1، 2]، [3، 4]]\nنتيجة = ق[1][0]").unwrap();
    assert_eq!(r.globals.get("نتيجة"), Some(&Value::Integer(3)));
}

#[test]
fn test_59_dict_nested() {
    let r = run_arabi("ف = {\"ا\": {\"ب\": 42}}\nنتيجة = ف[\"ا\"][\"ب\"]").unwrap();
    assert_eq!(r.globals.get("نتيجة"), Some(&Value::Integer(42)));
}

#[test]
fn test_60_list_comprehension_with_condition() {
    let r = run_arabi("ق = [1، 2، 3، 4، 5، 6]\nز = [ع * 2 لكل ع في ق اذا ع > 3]").unwrap();
    match r.globals.get("ز").unwrap() {
        Value::List(items) => {
            let b = items.borrow();
            assert_eq!(*b, vec![Value::Integer(8), Value::Integer(10), Value::Integer(12)]);
        }
        _ => panic!("Expected List"),
    }
}

#[test]
fn test_61_context_manager_file() {
    let _ = std::fs::write("test_cm.txt", "مرحبا");
    let r = run_arabi("باستخدام افتح(\"test_cm.txt\"، \"ق\") بشرط ملف:\n    الناتج = ملف.اقرا()").unwrap();
    assert_eq!(r.globals.get("الناتج"), Some(&Value::String(Rc::new("مرحبا".to_string()))));
    let _ = std::fs::remove_file("test_cm.txt");
}

#[test]
fn test_62_generator_basic() {
    let r = run_arabi("دالة عدّاد(ن):\n    س = 0\n    بينما س < ن:\n        سلم(س)\n        س += 1\nالنتيجة = []\nلكل ع في عدّاد(3):\n    النتيجة.اضف(ع)").unwrap();
    match r.globals.get("النتيجة").unwrap() {
        Value::List(items) => {
            let b = items.borrow();
            assert_eq!(*b, vec![Value::Integer(0), Value::Integer(1), Value::Integer(2)]);
        }
        _ => panic!("Expected List"),
    }
}

#[test]
fn test_64_math_operations() {
    assert_eq!(run_get("نتيجة = 2 ^ 10", "نتيجة"), Value::Integer(1024));
    assert_eq!(run_get("نتيجة2 = 20 \\ 3", "نتيجة2"), Value::Integer(6));
    assert_eq!(run_get("نتيجة3 = 20 % 3", "نتيجة3"), Value::Integer(2));
}

#[test]
fn test_65_set_operations() {
    let r = run_arabi("م1 = مميزة([1، 2، 3])\nم2 = مميزة([2، 3، 4])\nم3 = م1 + م2").unwrap();
    match r.globals.get("م3").unwrap() {
        Value::Set(items) => {
            let b = items.borrow();
            assert_eq!(b.len(), 4);
        }
        _ => panic!("Expected Set"),
    }
}

#[test]
fn test_66_negative_numbers() {
    assert_eq!(run_get("س = -5\nنتيجة = س * 2", "نتيجة"), Value::Integer(-10));
}

#[test]
fn test_67_boolean_logic() {
    assert_eq!(run_get("نتيجة = صح و ليس خطا", "نتيجة"), Value::Boolean(true));
    assert_eq!(run_get("نتيجة2 = خطا او صح", "نتيجة2"), Value::Boolean(true));
    assert_eq!(run_get("نتيجة3 = ليس صح", "نتيجة3"), Value::Boolean(false));
}

#[test]
fn test_68_type_checking() {
    assert_eq!(run_get("نتيجة = نوع(5)", "نتيجة"), Value::String(Rc::new("صحيح".to_string())));
    assert_eq!(run_get("نتيجة2 = نوع(\"مرحبا\")", "نتيجة2"), Value::String(Rc::new("نص".to_string())));
    assert_eq!(run_get("نتيجة3 = نوع(3.14)", "نتيجة3"), Value::String(Rc::new("عشري".to_string())));
}

#[test]
fn test_69_builtin_len() {
    assert_eq!(run_get("نتيجة = طول([1، 2، 3])", "نتيجة"), Value::Integer(3));
    assert_eq!(run_get("نتيجة2 = طول(\"مرحبا\")", "نتيجة2"), Value::Integer(5));
    assert_eq!(run_get("نتيجة3 = طول({\"ا\": 1، \"ب\": 2})", "نتيجة3"), Value::Integer(2));
}

#[test]
fn test_70_multi_level_closures_modify() {
    let r = run_arabi("دالة صانع():\n    ع = 0\n    دالة عداد():\n        محلي ع\n        ع += 1\n        ارجع ع\n    ارجع عداد\nع = صانع()\nا1 = ع()\nا2 = ع()\nا3 = ع()\nا4 = ع()\nا5 = ع()").unwrap();
    assert_eq!(r.globals.get("ا5"), Some(&Value::Integer(5)));
}

// ============================================================================
// 23. Generator send()
// ============================================================================

#[test]
fn test_71_generator_send() {
    let r = run_arabi("دالة عدّاد():\n    س = 0\n    بينما صح:\n        قيمة = سلم(س)\n        س = قيمة + 1\n\nم = عدّاد()\nا1 = م()\nا2 = ابعث(م، 10)\nا3 = ابعث(م، 20)").unwrap();
    assert_eq!(r.globals.get("ا1"), Some(&Value::Integer(0)));
    assert_eq!(r.globals.get("ا2"), Some(&Value::Integer(11)));
    assert_eq!(r.globals.get("ا3"), Some(&Value::Integer(21)));
}

// ============================================================================
// 24. New Builtins
// ============================================================================

#[test]
fn test_72_ahdof_element() {
    assert_eq!(run_get("ق = [1، 2، 3]\nن = احذف_عنصر(ق)\nن", "ن"), Value::Integer(3));
}

#[test]
fn test_72b_ahdof_element_index() {
    assert_eq!(run_get("ق = [1، 2، 3]\nن = احذف_عنصر(ق، 0)\nن", "ن"), Value::Integer(1));
}

#[test]
fn test_72c_ahotod() {
    assert_eq!(
        run_get("ق = [1، 2]\nاحطظ(ق، 3)\nن = طول(ق)\nن", "ن"),
        Value::Integer(3)
    );
}

#[test]
fn test_72d_adokhol_fi() {
    assert_eq!(
        run_get("ق = [1، 3]\nادخل_في(ق، 1، 2)\nن = ق[1]", "ن"),
        Value::Integer(2)
    );
}

#[test]
fn test_72e_ahdof_value() {
    assert_eq!(
        run_get("ق = [1، 2، 3، 2]\nن = احذف_قيمة(ق، 2)\nن", "ن"),
        Value::Boolean(true)
    );
}

#[test]
fn test_72f_ahdof_value_not_found() {
    assert_eq!(
        run_get("ق = [1، 2، 3]\nن = احذف_قيمة(ق، 5)\nن", "ن"),
        Value::Boolean(false)
    );
}

#[test]
fn test_72g_ikhtizal() {
    assert_eq!(
        run_get("ن = اختزال(خطية أ، ب : أ + ب، [1، 2، 3، 4])\nن", "ن"),
        Value::Integer(10)
    );
}

#[test]
fn test_72h_ikhtizal_init() {
    assert_eq!(
        run_get("ن = اختزال(خطية أ، ب : أ + ب، [1، 2، 3]، 10)\nن", "ن"),
        Value::Integer(16)
    );
}

// ============================================================================
// 25. Functional Programming Builtins
// ============================================================================

#[test]
fn test_73_mosattah() {
    match run_get("ن = مسطح([[1، 2]، [3، [4، 5]]])\nن", "ن") {
        Value::List(items) => {
            let vals: Vec<i64> = items.borrow().iter().map(|v| match v {
                Value::Integer(n) => *n,
                _ => panic!("Expected Integer"),
            }).collect();
            assert_eq!(vals, vec![1, 2, 3, 4, 5]);
        }
        _ => panic!("Expected List"),
    }
}

#[test]
fn test_74_dakhm() {
    match run_get("ن = ضخم(خطية ج : [ج، ج * 10]، [1، 2، 3])\nن", "ن") {
        Value::List(items) => {
            let vals: Vec<i64> = items.borrow().iter().map(|v| match v {
                Value::Integer(n) => *n,
                _ => panic!("Expected Integer"),
            }).collect();
            assert_eq!(vals, vec![1, 10, 2, 20, 3, 30]);
        }
        _ => panic!("Expected List"),
    }
}

#[test]
fn test_75_idmaj_fahras() {
    match run_get("م = [\"أ\"، \"ب\"، \"ج\"]\nق = [1، 2، 3]\nن = ادمج_فهرس_بـ(م، ق)\nن", "ن") {
        Value::Dict(d) => {
            let pairs = d.borrow();
            assert_eq!(pairs.len(), 3);
        }
        _ => panic!("Expected Dict"),
    }
}

#[test]
fn test_76_tajmee() {
    match run_get("ن = تجميع(خطية ج : ج > 2، [1، 2، 3، 4، 5])\nن", "ن") {
        Value::Dict(d) => {
            let pairs = d.borrow();
            assert_eq!(pairs.len(), 2);
        }
        _ => panic!("Expected Dict"),
    }
}

#[test]
fn test_77_adad_takrar() {
    match run_get("ن = عدد_تكرار([\"أ\"، \"ب\"، \"أ\"، \"ج\"، \"أ\"])\nن", "ن") {
        Value::Dict(d) => {
            let pairs = d.borrow();
            assert_eq!(pairs.len(), 3);
        }
        _ => panic!("Expected Dict"),
    }
}

#[test]
fn test_78_tajziya_list() {
    match run_get("ن = تجزئة_قائمة([1، 2، 3، 4، 5]، 2)\nن", "ن") {
        Value::List(chunks) => {
            let list = chunks.borrow();
            assert_eq!(list.len(), 3);
            match &list[0] {
                Value::List(first) => assert_eq!(first.borrow().len(), 2),
                _ => panic!("Expected sub-list"),
            }
        }
        _ => panic!("Expected List"),
    }
}

#[test]
fn test_79_afasal() {
    match run_get("ن = افصل(خطية ج : ج % 2 == 0، [1، 2، 3، 4، 5، 6])\nن", "ن") {
        Value::List(result) => {
            let list = result.borrow();
            assert_eq!(list.len(), 2);
            match &list[0] {
                Value::List(even) => assert_eq!(even.borrow().len(), 3),
                _ => panic!("Expected sub-list"),
            }
            match &list[1] {
                Value::List(odd) => assert_eq!(odd.borrow().len(), 3),
                _ => panic!("Expected sub-list"),
            }
        }
        _ => panic!("Expected List"),
    }
}

#[test]
fn test_80_exp_functions() {
    let r = run_get("ن = أس(0)", "ن");
    assert_eq!(r, Value::Float(1.0));
    let r = run_get("ن = أس2(10)", "ن");
    assert_eq!(r, Value::Float(1024.0));
    let r = run_get("ن = أس10(3)", "ن");
    assert_eq!(r, Value::Float(1000.0));
}

#[test]
fn test_81_cbrt_and_sign() {
    let r = run_get("ن = جذر_ثلاثي(27)", "ن");
    assert_eq!(r, Value::Float(3.0));
    let r = run_get("ن = جذر_ثلاثي(-8)", "ن");
    assert_eq!(r, Value::Float(-2.0));
    let r = run_get("ن = سلب(5)", "ن");
    assert_eq!(r, Value::Integer(-5));
    let r = run_get("ن = سلب(-3)", "ن");
    assert_eq!(r, Value::Integer(3));
}

#[test]
fn test_82_constants_and_special() {
    let r = run_get("م = لاشي()", "م");
    assert_eq!(r, Value::Null);
}

#[test]
fn test_83_regex_module() {
    // طابق (match)
    let r = run_arabi("استورد نمط\nن = نمط.تطابق(\"^[a-z]+$\", \"hello\")").unwrap();
    assert_eq!(r.globals.get("ن").cloned().unwrap(), Value::Boolean(true));

    let r = run_arabi("استورد نمط\nن = نمط.تطابق(\"^[0-9]+$\", \"abc\")").unwrap();
    assert_eq!(r.globals.get("ن").cloned().unwrap(), Value::Boolean(false));

    // ابحث (search) - dict access with ["key"]
    let r = run_arabi("استورد نمط\nم = نمط.ابحث(\"\\\\d+\", \"there are 3 cats\")\nن = م[\"بداية\"]").unwrap();
    assert_eq!(r.globals.get("ن").cloned().unwrap(), Value::Integer(10));

    // استبدل (replace)
    let r = run_arabi("استورد نمط\nن = نمط.استبدل(\"cat\", \"the cat sat on the cat\", \"dog\")").unwrap();
    assert_eq!(r.globals.get("ن").cloned().unwrap(), Value::String(Rc::new("the dog sat on the dog".to_string())));

    // قسم (split)
    let r = run_arabi("استورد نمط\nق = نمط.قسم(\"[,;]\", \"a,b;c\")\nن = طول(ق)").unwrap();
    assert_eq!(r.globals.get("ن").cloned().unwrap(), Value::Integer(3));
}

#[test]
fn test_84_datetime_functions() {
    // سنة/شهر/يوم should return integers
    let r = run_arabi("ن = سنة()").unwrap();
    assert!(matches!(r.globals.get("ن").cloned().unwrap(), Value::Integer(y) if y >= 2024));

    let r = run_arabi("ن = شهر()").unwrap();
    assert!(matches!(r.globals.get("ن").cloned().unwrap(), Value::Integer(m) if m >= 1 && m <= 12));

    let r = run_arabi("ن = يوم()").unwrap();
    assert!(matches!(r.globals.get("ن").cloned().unwrap(), Value::Integer(d) if d >= 1 && d <= 31));

    // ساعة/دقيقة/ثانية
    let r = run_arabi("ن = ساعة()").unwrap();
    assert!(matches!(r.globals.get("ن").cloned().unwrap(), Value::Integer(h) if h >= 0 && h <= 23));

    let r = run_arabi("ن = دقيقة()").unwrap();
    assert!(matches!(r.globals.get("ن").cloned().unwrap(), Value::Integer(m) if m >= 0 && m <= 59));

    // يوم_الاسبوع
    let r = run_arabi("ن = يوم_الاسبوع()").unwrap();
    assert!(matches!(r.globals.get("ن").cloned().unwrap(), Value::String(_)));

    // هل_سنة_كبيسة
    let r = run_arabi("ن = هل_سنة_كبيسة(2024)").unwrap();
    assert_eq!(r.globals.get("ن").cloned().unwrap(), Value::Boolean(true));
    let r = run_arabi("ن = هل_سنة_كبيسة(2023)").unwrap();
    assert_eq!(r.globals.get("ن").cloned().unwrap(), Value::Boolean(false));

    // ايام_الشهر
    let r = run_arabi("ن = ايام_الشهر(2، 2024)").unwrap();
    assert_eq!(r.globals.get("ن").cloned().unwrap(), Value::Integer(29));
    let r = run_arabi("ن = ايام_الشهر(1)").unwrap();
    assert_eq!(r.globals.get("ن").cloned().unwrap(), Value::Integer(31));
}

#[test]
fn test_85_encoding_and_collections() {
    // base64 encode/decode
    let r = run_arabi("ن = تشفير_64(\"مرحبا\")").unwrap();
    assert!(matches!(r.globals.get("ن").cloned().unwrap(), Value::String(_)));

    let r = run_arabi("م = تشفير_64(\"test\")\nن = فك_تشفير_64(م)").unwrap();
    assert_eq!(r.globals.get("ن").cloned().unwrap(), Value::String(Rc::new("test".to_string())));

    // hex encode/decode
    let r = run_arabi("ن = تشفير_سداسي(\"AB\")").unwrap();
    assert_eq!(r.globals.get("ن").cloned().unwrap(), Value::String(Rc::new("4142".to_string())));

    let r = run_arabi("ن = فك_تشفير_سداسي(\"4142\")").unwrap();
    assert_eq!(r.globals.get("ن").cloned().unwrap(), Value::String(Rc::new("AB".to_string())));

    // url encode
    let r = run_arabi("ن = تشفير_رابط(\"hello world\")").unwrap();
    assert_eq!(r.globals.get("ن").cloned().unwrap(), Value::String(Rc::new("hello%20world".to_string())));

    // collections: ادمج, موجود, كرر
    let r = run_arabi("ن = ادمج([1، 2]، [3، 4])").unwrap();
    assert_eq!(r.globals.get("ن").cloned().unwrap(), Value::List(SharedList::new(vec![Value::Integer(1), Value::Integer(2), Value::Integer(3), Value::Integer(4)])));

    let r = run_arabi("ن = موجود(2، [1، 2، 3])").unwrap();
    assert_eq!(r.globals.get("ن").cloned().unwrap(), Value::Boolean(true));

    let r = run_arabi("ن = موجود(5، [1، 2، 3])").unwrap();
    assert_eq!(r.globals.get("ن").cloned().unwrap(), Value::Boolean(false));

    let r = run_arabi("ن = طول(كرر(\"x\"، 3))").unwrap();
    assert_eq!(r.globals.get("ن").cloned().unwrap(), Value::Integer(3));
}

#[test]
fn test_86_math_builtins() {
    // حاصل_ضرب
    let r = run_arabi("ن = حاصل_ضرب([2، 3، 4])").unwrap();
    assert_eq!(r.globals.get("ن").cloned().unwrap(), Value::Integer(24));

    let r = run_arabi("ن = حاصل_ضرب([1، 1، 1])").unwrap();
    assert_eq!(r.globals.get("ن").cloned().unwrap(), Value::Integer(1));

    // نسبة
    let r = run_arabi("ن = نسبة(25، 100)").unwrap();
    assert_eq!(r.globals.get("ن").cloned().unwrap(), Value::Float(25.0));

    let r = run_arabi("ن = نسبة(1، 3)").unwrap();
    assert!(matches!(r.globals.get("ن").cloned().unwrap(), Value::Float(f) if (f - 33.333).abs() < 0.01));

    // تقريب_ل
    let r = run_arabi("ن = تقريب_ل(3.14159، 2)").unwrap();
    assert_eq!(r.globals.get("ن").cloned().unwrap(), Value::Float(3.14));

    // علامة
    let r = run_arabi("ن = علامة(5)").unwrap();
    assert_eq!(r.globals.get("ن").cloned().unwrap(), Value::Integer(1));
    let r = run_arabi("ن = علامة(-3)").unwrap();
    assert_eq!(r.globals.get("ن").cloned().unwrap(), Value::Integer(-1));
    let r = run_arabi("ن = علامة(0)").unwrap();
    assert_eq!(r.globals.get("ن").cloned().unwrap(), Value::Integer(0));
}

#[test]
fn test_87_bitwise_ops() {
    // BitAnd
    let r = run_arabi("ن = 12 & 10").unwrap();
    assert_eq!(r.globals.get("ن").cloned().unwrap(), Value::Integer(8));

    // BitOr
    let r = run_arabi("ن = 12 | 10").unwrap();
    assert_eq!(r.globals.get("ن").cloned().unwrap(), Value::Integer(14));

    // Shl
    let r = run_arabi("ن = 1 << 4").unwrap();
    assert_eq!(r.globals.get("ن").cloned().unwrap(), Value::Integer(16));

    // Shr
    let r = run_arabi("ن = 32 >> 3").unwrap();
    assert_eq!(r.globals.get("ن").cloned().unwrap(), Value::Integer(4));

    // BitNot
    let r = run_arabi("ن = ~0").unwrap();
    assert_eq!(r.globals.get("ن").cloned().unwrap(), Value::Integer(-1));

    let r = run_arabi("ن = ~255").unwrap();
    assert_eq!(r.globals.get("ن").cloned().unwrap(), Value::Integer(-256));

    // Chained bitwise
    let r = run_arabi("ن = (12 & 10) | 5").unwrap();
    assert_eq!(r.globals.get("ن").cloned().unwrap(), Value::Integer(13));

    // Precedence: & binds tighter than |
    let r = run_arabi("ن = 12 | 3 & 1").unwrap();
    assert_eq!(r.globals.get("ن").cloned().unwrap(), Value::Integer(13));

    // Shl/Shr with larger values
    let r = run_arabi("ن = 1 << 10").unwrap();
    assert_eq!(r.globals.get("ن").cloned().unwrap(), Value::Integer(1024));

    let r = run_arabi("ن = 1024 >> 5").unwrap();
    assert_eq!(r.globals.get("ن").cloned().unwrap(), Value::Integer(32));

    // Bitwise with variables
    let r = run_arabi("ع = 255\nن = ع & 15").unwrap();
    assert_eq!(r.globals.get("ن").cloned().unwrap(), Value::Integer(15));

    let r = run_arabi("م = 6\nن = م << 2").unwrap();
    assert_eq!(r.globals.get("ن").cloned().unwrap(), Value::Integer(24));
}

#[test]
fn test_88_error_handling_no_panic() {
    let result = run_arabi("ن = 10 / 0");
    assert!(result.is_err(), "Division by zero should return an error");

    let result = run_arabi("ن = 10 % 0");
    assert!(result.is_err(), "Modulo by zero should return an error");

    let result = run_arabi("ن = 2 ** 1000");
    assert!(result.is_ok(), "Large power should not panic");
}

#[test]
fn test_89_exception_preserves_line_info() {
    let source = "ن = 10 / 0";
    let result = run_arabi(source);
    assert!(result.is_err());
    let err_msg = result.unwrap_err();
    assert!(err_msg.contains("خطا") || err_msg.contains("قسمة") || err_msg.contains("error"), "Error message: {}", err_msg);
}

#[test]
fn test_90_exception_try_except() {
    let source = "المحاولة = \"فشل\"\nحاول:\n    ن = 10 / 0\nخلل:\n    المحاولة = \"نجاح\"";
    let r = run_arabi(source).unwrap();
    assert_eq!(r.globals.get("المحاولة").cloned().unwrap(), Value::String(Rc::new("نجاح".to_string())));
}

#[test]
fn test_91_exception_finally() {
    let source = "النتيجة = \"\"\nحاول:\n    ن = 10 / 0\nخلل:\n    النتيجة = النتيجة + \"أ\"\nنهاية:\n    النتيجة = النتيجة + \"ب\"";
    let r = run_arabi(source).unwrap();
    assert_eq!(r.globals.get("النتيجة").cloned().unwrap(), Value::String(Rc::new("أب".to_string())));
}

#[test]
fn test_92_exception_nested_try() {
    let source = "النتيجة = \"\"\nحاول:\n    حاول:\n        ن = 10 / 0\n    خلل:\n        النتيجة = النتيجة + \"داخلي\"\nخلل:\n    النتيجة = النتيجة + \"خارجي\"";
    let r = run_arabi(source).unwrap();
    assert_eq!(r.globals.get("النتيجة").cloned().unwrap(), Value::String(Rc::new("داخلي".to_string())));
}

#[test]
fn test_93_exception_raise_and_catch() {
    // Runtime error (division by zero) caught by خلل block
    let source = "النتيجة = 0\nحاول:\n    س = 1 / 0\n    النتيجة = 1\nخلل:\n    النتيجة = 2";
    let r = run_arabi(source).unwrap();
    assert_eq!(r.globals.get("النتيجة").cloned().unwrap(), Value::Integer(2));

    // خطا_ن creates exception value without throwing
    let source2 = "ح = خطا_ن(\"استثناء_خطا\", \"رسالة\")\nالن = \"تم\"";
    let r2 = run_arabi(source2).unwrap();
    assert_eq!(r2.globals.get("الن").cloned().unwrap(), Value::String(Rc::new("تم".to_string())));
}

#[test]
fn test_94_enumerate() {
    let source = "ق = [10, 20, 30]\nن =عد(ق)";
    let r = run_arabi(source).unwrap();
    let val = r.globals.get("ن").cloned().unwrap();
    match val {
        Value::List(items) => {
            let items = items.borrow();
            assert_eq!(items.len(), 3);
            // Each item should be a tuple (index, value)
            match &items[0] {
                Value::Tuple(t) => {
                    assert_eq!(t[0], Value::Integer(0));
                    assert_eq!(t[1], Value::Integer(10));
                }
                other => panic!("Expected tuple, got {:?}", other),
            }
            match &items[2] {
                Value::Tuple(t) => {
                    assert_eq!(t[0], Value::Integer(2));
                    assert_eq!(t[1], Value::Integer(30));
                }
                other => panic!("Expected tuple, got {:?}", other),
            }
        }
        other => panic!("Expected list, got {:?}", other),
    }
}

#[test]
fn test_95_enumerate_start() {
    let source = "ق = [10, 20]\nن =عد(ق, 5)";
    let r = run_arabi(source).unwrap();
    let val = r.globals.get("ن").cloned().unwrap();
    match val {
        Value::List(items) => {
            let items = items.borrow();
            assert_eq!(items.len(), 2);
            match &items[0] {
                Value::Tuple(t) => {
                    assert_eq!(t[0], Value::Integer(5));
                    assert_eq!(t[1], Value::Integer(10));
                }
                other => panic!("Expected tuple, got {:?}", other),
            }
            match &items[1] {
                Value::Tuple(t) => {
                    assert_eq!(t[0], Value::Integer(6));
                    assert_eq!(t[1], Value::Integer(20));
                }
                other => panic!("Expected tuple, got {:?}", other),
            }
        }
        other => panic!("Expected list, got {:?}", other),
    }
}

#[test]
fn test_96_zip() {
    let source = "ق1 = [1, 2]\nق2 = [10, 20]\nن = اقتران(ق1, ق2)";
    let r = run_arabi(source).unwrap();
    let val = r.globals.get("ن").cloned().unwrap();
    match val {
        Value::List(items) => {
            let items = items.borrow();
            assert_eq!(items.len(), 2);
            match &items[0] {
                Value::Tuple(t) => {
                    assert_eq!(t[0], Value::Integer(1));
                    assert_eq!(t[1], Value::Integer(10));
                }
                other => panic!("Expected tuple, got {:?}", other),
            }
            match &items[1] {
                Value::Tuple(t) => {
                    assert_eq!(t[0], Value::Integer(2));
                    assert_eq!(t[1], Value::Integer(20));
                }
                other => panic!("Expected tuple, got {:?}", other),
            }
        }
        other => panic!("Expected list, got {:?}", other),
    }
}

#[test]
fn test_97_zip_three_lists() {
    let source = "ن = اقتران([1, 2], [10, 20], [100, 200])";
    let r = run_arabi(source).unwrap();
    let val = r.globals.get("ن").cloned().unwrap();
    match val {
        Value::List(items) => {
            let items = items.borrow();
            assert_eq!(items.len(), 2);
            match &items[0] {
                Value::Tuple(t) => {
                    assert_eq!(t.len(), 3);
                    assert_eq!(t[0], Value::Integer(1));
                    assert_eq!(t[1], Value::Integer(10));
                    assert_eq!(t[2], Value::Integer(100));
                }
                other => panic!("Expected tuple, got {:?}", other),
            }
        }
        other => panic!("Expected list, got {:?}", other),
    }
}

#[test]
fn test_98_reduce() {
    let source = "ق = [1، 2، 3، 4]\nن = اختزل(خطية أ، ب : أ + ب، ق)";
    let r = run_arabi(source).unwrap();
    assert_eq!(r.globals.get("ن").cloned().unwrap(), Value::Integer(10));
}

#[test]
fn test_99_reduce_initial() {
    let source = "ق = [1، 2، 3]\nن = اختزل(خطية أ، ب : أ + ب، ق، 10)";
    let r = run_arabi(source).unwrap();
    assert_eq!(r.globals.get("ن").cloned().unwrap(), Value::Integer(16));
}

#[test]
fn test_100_decorator_basic() {
    let source = "دالة مزخرف(د):\n    ارجع د\nزخرف مزخرف\nدالة تحية():\n    ارجع \"مرحبا\"\nالنتيجة = تحية()";
    let r = run_arabi(source).unwrap();
    let val = r.globals.get("النتيجة").cloned().unwrap();
    match val {
        Value::String(s) => assert!(s.contains("مرحبا"), "Got: {}", s),
        other => panic!("Expected String, got {:?}", other),
    }
}

#[test]
fn test_101_decorator_at_symbol() {
    let source = "دالة مزخرف2(د):\n    ارجع د\n@مزخرف2\nدالة تحية2():\n    ارجع \"عالم\"\nالناتج = تحية2()";
    let r = run_arabi(source).unwrap();
    let val = r.globals.get("الناتج").cloned().unwrap();
    match val {
        Value::String(s) => assert!(s.contains("عالم"), "Got: {}", s),
        other => panic!("Expected String, got {:?}", other),
    }
}

#[test]
fn test_102_with_statement_as() {
    let _ = std::fs::write("test_with1.txt", "اختبار");
    let source = "باستخدام افتح(\"test_with1.txt\"، \"ق\") بشرط م:\n    النتيجة = م.اقرا()";
    let r = run_arabi(source).unwrap();
    assert_eq!(r.globals.get("النتيجة"), Some(&Value::String(Rc::new("اختبار".to_string()))));
    let _ = std::fs::remove_file("test_with1.txt");
}

#[test]
fn test_103_with_statement_no_target() {
    let _ = std::fs::write("test_with2.txt", "نص");
    let source = "باستخدام افتح(\"test_with2.txt\"، \"ق\"):\n    النتيجة2 = \"تم\"";
    let r = run_arabi(source).unwrap();
    assert_eq!(r.globals.get("النتيجة2"), Some(&Value::String(Rc::new("تم".to_string()))));
    let _ = std::fs::remove_file("test_with2.txt");
}

#[test]
fn test_104_global_keyword() {
    let source = "س = 0\nدالة عداد():\n    عام س\n    س += 1\n\nعداد()\nعداد()\nعداد()\nالناتج3 = س";
    let r = run_arabi(source).unwrap();
    assert_eq!(r.globals.get("الناتج3"), Some(&Value::Integer(3)));
}

#[test]
fn test_105_nonlocal_keyword() {
    let source = "دالة صانع():\n    ع = 0\n    دالة عداد():\n        محلي ع\n        ع += 1\n        ارجع ع\n    ارجع عداد\nص = صانع()\nا1 = ص()\nا2 = ص()\nا3 = ص()\nالناتج4 = ا3";
    let r = run_arabi(source).unwrap();
    assert_eq!(r.globals.get("الناتج4"), Some(&Value::Integer(3)));
}

#[test]
fn test_106_import_math() {
    let source = "استورد حساب\nالناتج5 = حساب.جذر(144)";
    let r = run_arabi(source).unwrap();
    assert_eq!(r.globals.get("الناتج5"), Some(&Value::Float(12.0)));
}

#[test]
fn test_107_import_random() {
    let source = "استورد عشوائي\nن = عشوائي.عشوائي_صحيح(1، 100)\nالصحيح = (ن >= 1 و ن <= 100)";
    let r = run_arabi(source).unwrap();
    assert_eq!(r.globals.get("الصحيح"), Some(&Value::Boolean(true)));
}

#[test]
fn test_108_import_regex() {
    let source = "استورد نمط\nالناتج6 = نمط.تطابق(\"^[a-z]+$\"، \"hello\")";
    let r = run_arabi(source).unwrap();
    assert_eq!(r.globals.get("الناتج6"), Some(&Value::Boolean(true)));
}

#[test]
fn test_109_assert_true() {
    let source = "اكد صح\nالناتج7 = 1";
    let r = run_arabi(source).unwrap();
    assert_eq!(r.globals.get("الناتج7"), Some(&Value::Integer(1)));
}

#[test]
fn test_110_assert_false_raises() {
    let source = "حاول:\n    اكد خطا\n    الناتج8 = 1\nخلل:\n    الناتج8 = 2";
    let r = run_arabi(source).unwrap();
    assert_eq!(r.globals.get("الناتج8"), Some(&Value::Integer(2)));
}

#[test]
fn test_111_assert_with_message() {
    let source = "اكد 5 == 5، \"يجب ان يساوي 5\"\nالناتج9 = 10";
    let r = run_arabi(source).unwrap();
    assert_eq!(r.globals.get("الناتج9"), Some(&Value::Integer(10)));
}

#[test]
fn test_112_delete_statement() {
    let source = "س2 = 42\nاحذف س2";
    let r = run_arabi(source).unwrap();
    match r.globals.get("س2") {
        None => {},
        Some(Value::Null) => {},
        other => panic!("Expected deleted (None) or Null, got {:?}", other),
    }
}

#[test]
fn test_113_walrus_operator() {
    let source = "س3 = 0\nس3 = (م2 := 42)\nالناتج11 = م2";
    let r = run_arabi(source).unwrap();
    assert_eq!(r.globals.get("الناتج11"), Some(&Value::Integer(42)));
}

#[test]
fn test_114_string_methods_replace_split_trim() {
    let source = "ن1 = \"مرحبا بالعالم\".استبدل(\"العالم\"، \"يا عالم\")\nن2 = \"ا،ب،ج\".افصل(\"،\")\nن3 = \"  مرحب  \".شطب()";
    let r = run_arabi(source).unwrap();
    assert_eq!(r.globals.get("ن1"), Some(&Value::String(Rc::new("مرحبا بيا عالم".to_string()))));
    let n2 = r.globals.get("ن2").unwrap();
    let n2_len = match n2 {
        Value::Tuple(items) => items.len(),
        Value::List(items) => items.borrow().len(),
        other => panic!("Expected Tuple or List, got {:?}", other),
    };
    assert_eq!(n2_len, 3);
    assert_eq!(r.globals.get("ن3"), Some(&Value::String(Rc::new("مرحب".to_string()))));
}

#[test]
fn test_115_string_methods_startswith_endswith_contains() {
    let source = "ن4 = \"مرحبا بالعالم\".يبدا(\"مرحبا\")\nن5 = \"مرحبا بالعالم\".ينتهي(\"العالم\")\nن6 = \"مرحبا بالعالم\".يحتوي(\"بالعالم\")";
    let r = run_arabi(source).unwrap();
    assert_eq!(r.globals.get("ن4"), Some(&Value::Boolean(true)));
    assert_eq!(r.globals.get("ن5"), Some(&Value::Boolean(true)));
    assert_eq!(r.globals.get("ن6"), Some(&Value::Boolean(true)));
}

#[test]
fn test_116_star_unpack_in_assignment() {
    let source = "ا2، *ب2 = [10، 20، 30]\nالناتج12 = ا2 + ب2[0]";
    let r = run_arabi(source).unwrap();
    assert_eq!(r.globals.get("الناتج12"), Some(&Value::Integer(30)));
}

#[test]
fn test_117_star_in_function_call() {
    let source = "دالة جمع(*ص):\n    ن = 0\n    لكل ع في ص:\n        ن += ع\n    ارجع ن\nق2 = [1، 2، 3]\nالناتج13 = جمع(*ق2)";
    let r = run_arabi(source).unwrap();
    assert_eq!(r.globals.get("الناتج13"), Some(&Value::Integer(6)));
}

#[test]
fn test_118_dict_comprehension() {
    let source = "النتيجة5 = {س: س ^ 2 لكل س في [1، 2، 3]}";
    let r = run_arabi(source).unwrap();
    match r.globals.get("النتيجة5").unwrap() {
        Value::Dict(pairs) => {
            let p = pairs.borrow();
            assert_eq!(p.len(), 3);
        }
        other => panic!("Expected Dict, got {:?}", other),
    }
}

#[test]
fn test_119_set_comprehension() {
    let source = "النتيجة6 = {س * 2 لكل س في [1، 2، 3، 2، 1]}";
    let r = run_arabi(source).unwrap();
    match r.globals.get("النتيجة6").unwrap() {
        Value::Set(items) => {
            let s = items.borrow();
            assert_eq!(s.len(), 3);
        }
        other => panic!("Expected Set, got {:?}", other),
    }
}

#[test]
fn test_120_recursion_depth_limit() {
    let source = "دالة بحت_لانهائي(ن):\n    ن = ن + 1\n    م = بحت_لانهائي(ن)\n    م\nالناتج = \"بداية\"\nبحت_لانهائي(0)\nالناتج = \"نهاية\"";
    match run_arabi(source) {
        Ok(r) => {
            panic!("Expected error but got OK. الناتج = {:?}", r.globals.get("الناتج"));
        }
        Err(e) => {
            assert!(e.contains("تجاوز") || e.contains("استدعاء"), "Error should mention depth limit: {}", e);
        }
    }
}

#[test]
fn test_121_div_by_zero() {
    let source = "حاول:\n    الناتج = 10 / 0\nخلل:\n    الناتج = \"تم\"\nنهاية:\n    مرور";
    let r = run_arabi(source).unwrap();
    assert!(r.globals.get("الناتج").is_some());
}

#[test]
fn test_122_match_case_integer() {
    let source = r#"
ع = 2
طابق ع:
  حالة 1:
    الناتج = "واحد"
  حالة 2:
    الناتج = "اثنان"
  حالة 3:
    الناتج = "ثلاثة"
  حالة_اخرى:
    الناتج = "اخرى"
"#;
    let r = run_arabi(source).unwrap();
    assert_eq!(r.globals.get("الناتج"), Some(&Value::String(Rc::new("اثنان".to_string()))));
}

#[test]
fn test_123_match_case_default() {
    let source = r#"
ع = 10
طابق ع:
  حالة 1:
    الناتج = "واحد"
  حالة_اخرى:
    الناتج = "اخرى"
"#;
    let r = run_arabi(source).unwrap();
    assert_eq!(r.globals.get("الناتج"), Some(&Value::String(Rc::new("اخرى".to_string()))));
}

#[test]
fn test_124_match_case_string() {
    let source = r#"
اسم = "أحمد"
طابق اسم:
  حالة "أحمد":
    الناتج = "مرحبا أحمد"
  حالة "محمد":
    الناتج = "مرحبا محمد"
  حالة_اخرى:
    الناتج = "مرحبا غريب"
"#;
    let r = run_arabi(source).unwrap();
    assert_eq!(r.globals.get("الناتج"), Some(&Value::String(Rc::new("مرحبا أحمد".to_string()))));
}

#[test]
fn test_125_getattr_builtin() {
    let source = r#"
صنف نقطة:
  دالة __تهيئة__(هذا، ص، خ):
    هذا.ص = ص
    هذا.خ = خ

ن = نقطة(3، 4)
الناتج = خاصية(ن، "ص")
"#;
    let r = run_arabi(source).unwrap();
    assert_eq!(r.globals.get("الناتج"), Some(&Value::Integer(3)));
}

#[test]
fn test_126_setattr_builtin() {
    let source = r#"
صنف نقطة:
  دالة __تهيئة__(هذا، ص، خ):
    هذا.ص = ص
    هذا.خ = خ

ن = نقطة(3، 4)
تعيين_خاصية(ن، "س"، 30)
الناتج = خاصية(ن، "س")
"#;
    let r = run_arabi(source).unwrap();
    assert_eq!(r.globals.get("الناتج"), Some(&Value::Integer(30)));
}

#[test]
fn test_127_hasattr_builtin() {
    let source = r#"
صنف نقطة:
  دالة __تهيئة__(هذا، ص، خ):
    هذا.ص = ص
    هذا.خ = خ

ن = نقطة(3، 4)
م1 = هل_خاصية(ن، "ص")
م2 = هل_خاصية(ن، "لا_موجود")
"#;
    let r = run_arabi(source).unwrap();
    assert_eq!(r.globals.get("م1"), Some(&Value::Boolean(true)));
    assert_eq!(r.globals.get("م2"), Some(&Value::Boolean(false)));
}

#[test]
fn test_128_dot_attr_access() {
    let source = r#"
صنف نقطة:
  دالة __تهيئة__(هذا، ص، خ):
    هذا.ص = ص
    هذا.خ = خ

ن = نقطة(10، 20)
ن.ص = 100
الناتج = ن.ص
"#;
    let r = run_arabi(source).unwrap();
    assert_eq!(r.globals.get("الناتج"), Some(&Value::Integer(100)));
}

#[test]
fn test_129_subprocess_run() {
    let source = r#"
استورد عمليات
ن = عمليات.نفاذ("echo hello")
الناتج = ن["المخرجات"]
"#;
    let r = run_arabi(source).unwrap();
    match r.globals.get("الناتج") {
        Some(Value::String(s)) => assert!(s.trim().contains("hello")),
        other => panic!("Expected string containing 'hello', got {:?}", other),
    }
}

#[test]
fn test_130_subprocess_run_with_stdin() {
    let source = r#"
استورد عمليات
ن = عمليات.نفاذ_مع("more", "مرحبا")
الناتج = ن["المخرجات"]
"#;
    let r = run_arabi(source).unwrap();
    match r.globals.get("الناتج") {
        Some(Value::String(s)) => assert!(s.contains("مرحبا")),
        other => panic!("Expected string containing 'مرحبا', got {:?}", other),
    }
}

#[test]
fn test_131_subprocess_exit_code() {
    let source = r#"
استورد عمليات
ن = عمليات.نفاذ("exit /b 42")
الناتج = ن["الحالة"]
"#;
    let r = run_arabi(source).unwrap();
    assert_eq!(r.globals.get("الناتج"), Some(&Value::Integer(42)));
}

#[test]
fn test_132_subprocess_stderr() {
    let source = r#"
استورد عمليات
ن = عمليات.نفاذ("echo error 1>&2")
الناتج = ن["الاخطاء"]
"#;
    let r = run_arabi(source).unwrap();
    match r.globals.get("الناتج") {
        Some(Value::String(s)) => assert!(s.contains("error")),
        other => panic!("Expected string containing 'error', got {:?}", other),
    }
}

#[test]
fn test_133_subprocess_list() {
    let source = r#"
استورد عمليات
ن = عمليات.نفاذ_قائمة(["echo one", "echo two"])
العدد = طول(ن)
"#;
    let r = run_arabi(source).unwrap();
    assert_eq!(r.globals.get("العدد"), Some(&Value::Integer(2)));
}

#[test]
fn test_134_while_else() {
    let source = r#"
الناتج = 0
ن = 0
بينما ن < 5:
    الناتج = الناتج + ن
    ن = ن + 1
والا:
    الناتج = الناتج + 100
"#;
    let r = run_arabi(source).unwrap();
    assert_eq!(r.globals.get("الناتج"), Some(&Value::Integer(110)));
}

#[test]
fn test_135_while_else_continue() {
    let source = r#"
الناتج = 0
ن = 0
بينما ن < 5:
    ن = ن + 1
    اذا ن % 2 == 0:
        استمر
    الناتج = الناتج + ن
والا:
    الناتج = الناتج + 100
"#;
    let r = run_arabi(source).unwrap();
    assert_eq!(r.globals.get("الناتج"), Some(&Value::Integer(109)));
}

#[test]
fn test_136_for_else() {
    let source = r#"
الناتج = 0
لكل ن في [1، 2، 3]:
    الناتج = الناتج + ن
والا:
    الناتج = الناتج + 100
"#;
    let r = run_arabi(source).unwrap();
    assert_eq!(r.globals.get("الناتج"), Some(&Value::Integer(106)));
}

#[test]
fn test_137_for_else_continue() {
    let source = r#"
الناتج = 0
لكل ن في [1، 2، 3، 4، 5]:
    اذا ن % 2 == 0:
        استمر
    الناتج = الناتج + ن
والا:
    الناتج = الناتج + 100
"#;
    let r = run_arabi(source).unwrap();
    assert_eq!(r.globals.get("الناتج"), Some(&Value::Integer(109)));
}

#[test]
fn test_138_raise_explicit() {
    let source = r#"
حاول:
    ارم("خطأ مخصص")
خلل بشرط م:
    الناتج = "تم"
"#;
    let r = run_arabi(source).unwrap();
    match r.globals.get("الناتج") {
        Some(Value::String(s)) => assert_eq!(s.as_str(), "تم"),
        other => panic!("Expected 'تم', got {:?}", other),
    }
}

#[test]
fn test_139_callable_magic() {
    let source = r#"
صنف مكالمة:
    دالة __استدعاء__(هذا، م):
        ارجع م * 2
ن = مكالمة()
الناتج = ن(5)
"#;
    let r = run_arabi(source).unwrap();
    assert_eq!(r.globals.get("الناتج"), Some(&Value::Integer(10)));
}

#[test]
fn test_140_json_analyze() {
    let source = r#"
استورد كائن
الكائن = كائن.تحليل('{"الاسم": "أحمد"}')
الاسم = الكائن["الاسم"]
"#;
    let r = run_arabi(source).unwrap();
    match r.globals.get("الاسم") {
        Some(Value::String(s)) => assert_eq!(s.as_str(), "أحمد"),
        other => panic!("Expected 'أحمد', got {:?}", other),
    }
}

#[test]
fn test_141_json_convert() {
    let source = r#"
استورد كائن
القائمة = [1، 2، 3]
النص = كائن.تحويل(القائمة)
"#;
    let r = run_arabi(source).unwrap();
    match r.globals.get("النص") {
        Some(Value::String(s)) => assert!(s.contains("1"), "JSON should contain 1"),
        other => panic!("Expected JSON string, got {:?}", other),
    }
}

#[test]
fn test_142_context_manager() {
    let source = r#"
صنف سياق:
    دالة __ادخل__(هذا):
        هذا.الحالة = "داخل"
        ارجع هذا
    دالة __اترك__(هذا):
        هذا.الحالة = "خارج"
ن = سياق()
باستخدام ن:
    الناتج1 = ن.الحالة
الناتج2 = ن.الحالة
"#;
    let r = run_arabi(source).unwrap();
    assert_eq!(r.globals.get("الناتج1"), Some(&Value::String(Rc::new("داخل".to_string()))));
    assert_eq!(r.globals.get("الناتج2"), Some(&Value::String(Rc::new("خارج".to_string()))));
}

#[test]
fn test_143_sorted_with_key() {
    let source = r#"
القائمة = [3، 1، 4، 1، 5]
المرتب = مرتب(القائمة، خطية أ : -أ)
الناتج = المرتب[0]
"#;
    let r = run_arabi(source).unwrap();
    assert_eq!(r.globals.get("الناتج"), Some(&Value::Integer(5)));
}

#[test]
fn test_144_max_with_key() {
    let source = r#"
القائمة = [3، 1، 4، 1، 5]
الاكبر = اكبر(القائمة، خطية أ : أ * 2)
"#;
    let r = run_arabi(source).unwrap();
    assert_eq!(r.globals.get("الاكبر"), Some(&Value::Integer(5)));
}

#[test]
fn test_145_min_with_key() {
    let source = r#"
القائمة = [3، 1، 4، 1، 5]
الاصغر = اصغر(القائمة، خطية أ : -أ)
"#;
    let r = run_arabi(source).unwrap();
    assert_eq!(r.globals.get("الاصغر"), Some(&Value::Integer(5)));
}

#[test]
fn test_146_match_guard() {
    let source = r#"
ن = 5
طابق ن:
    حالة 5 عندما ن > 10:
        الناتج = "كبير"
    حالة 5 عندما ن > 3:
        الناتج = "متوسط"
    حالة_اخرى:
        الناتج = "صغير"
"#;
    let r = run_arabi(source).unwrap();
    assert_eq!(r.globals.get("الناتج"), Some(&Value::String(Rc::new("متوسط".to_string()))));
}

#[test]
fn test_147_match_guard_fallthrough() {
    let source = r#"
ن = 5
طابق ن:
    حالة 5 عندما ن > 10:
        الناتج = "كبير"
    حالة 5 عندما ن > 100:
        الناتج = "متوسط جداً"
    حالة_اخرى:
        الناتج = "الاخرى"
"#;
    let r = run_arabi(source).unwrap();
    assert_eq!(r.globals.get("الناتج"), Some(&Value::String(Rc::new("الاخرى".to_string()))));
}

#[test]
fn test_148_negative_list_indexing() {
    let source = r#"القائمة = [10، 20، 30، 40، 50]
الاخير = القائمة[-1]
الثالث = القائمة[-3]
الاول = القائمة[-5]"#;
    let r = run_arabi(source).unwrap();
    assert_eq!(r.globals.get("الاخير"), Some(&Value::Integer(50)));
    assert_eq!(r.globals.get("الثالث"), Some(&Value::Integer(30)));
    assert_eq!(r.globals.get("الاول"), Some(&Value::Integer(10)));
}

#[test]
fn test_149_negative_string_indexing() {
    let source = r#"النص = "مرحبا"
الاخير = النص[-1]
الثالث = النص[-3]"#;
    let r = run_arabi(source).unwrap();
    assert_eq!(r.globals.get("الاخير"), Some(&Value::String(Rc::new("ا".to_string()))));
    assert_eq!(r.globals.get("الثالث"), Some(&Value::String(Rc::new("ح".to_string()))));
}

#[test]
fn test_150_string_slicing() {
    let source = r#"النص = "مرحبا بالعالم"
جزء1 = النص[0:5]
جزء2 = النص[6:]
جزء3 = النص[:5]"#;
    let r = run_arabi(source).unwrap();
    assert_eq!(r.globals.get("جزء1"), Some(&Value::String(Rc::new("مرحبا".to_string()))));
    assert_eq!(r.globals.get("جزء2"), Some(&Value::String(Rc::new("بالعالم".to_string()))));
    assert_eq!(r.globals.get("جزء3"), Some(&Value::String(Rc::new("مرحبا".to_string()))));
}

#[test]
fn test_151_string_slicing_step() {
    let source = r#"النص = "abcdef"
جزء = النص[::2]"#;
    let r = run_arabi(source).unwrap();
    assert_eq!(r.globals.get("جزء"), Some(&Value::String(Rc::new("ace".to_string()))));
}

#[test]
fn test_152_string_slicing_negative() {
    let source = r#"النص = "مرحبا"
جزء = النص[-3:]"#;
    let r = run_arabi(source).unwrap();
    assert_eq!(r.globals.get("جزء"), Some(&Value::String(Rc::new("حبا".to_string()))));
}

#[test]
fn test_153_recursion_with_closure() {
    let source = r#"دالة صانع_عداد(بداية):
    عداد = بداية
    دالة عدّ():
        عداد = عداد + 1
        ارجع عداد
    ارجع عدّ

العداد = صانع_عداد(0)
ن1 = العداد()
ن2 = العداد()
ن3 = العداد()"#;
    let r = run_arabi(source).unwrap();
    assert_eq!(r.globals.get("ن1"), Some(&Value::Integer(1)));
    assert_eq!(r.globals.get("ن2"), Some(&Value::Integer(2)));
    assert_eq!(r.globals.get("ن3"), Some(&Value::Integer(3)));
}

#[test]
fn test_154_class_composition() {
    let source = r#"صنف نقطة:
    دالة __تهيئة__(هذا، ص، خ):
        هذا.ص = ص
        هذا.خ = خ
    دالة مسافة(هذا، ن):
        ص2 = ن.ص - هذا.ص
        خ2 = ن.خ - هذا.خ
        ارجع ص2 * ص2 + خ2 * خ2

ن1 = نقطة(0، 0)
ن2 = نقطة(3، 4)
الناتج = ن1.مسافة(ن2)"#;
    let r = run_arabi(source).unwrap();
    assert_eq!(r.globals.get("الناتج"), Some(&Value::Integer(25)));
}

#[test]
fn test_155_class_inheritance_method_override() {
    let source = r#"صنف حيوان:
    دالة __تهيئة__(هذا، اسم):
        هذا.اسم = اسم
    دالة صوت(هذا):
        ارجع "..."

صنف كلب(حيوان):
    دالة صوت(هذا):
        ارجع هذا.اسم + ": واف!"

صنف قط(حيوان):
    دالة صوت(هذا):
        ارجع هذا.اسم + ": نياو!"

ك = كلب("بودي")
ق = قط("ميوي")
ن1 = ك.صوت()
ن2 = ق.صوت()
ن3 = حيوان("??").صوت()"#;
    let r = run_arabi(source).unwrap();
    assert_eq!(r.globals.get("ن1"), Some(&Value::String(Rc::new("بودي: واف!".to_string()))));
    assert_eq!(r.globals.get("ن2"), Some(&Value::String(Rc::new("ميوي: نياو!".to_string()))));
    assert_eq!(r.globals.get("ن3"), Some(&Value::String(Rc::new("...".to_string()))));
}

#[test]
fn test_156_version_constant() {
    let source = r#"الناتج = الاصدار"#;
    let r = run_arabi(source).unwrap();
    assert_eq!(r.globals.get("الناتج"), Some(&Value::String(Rc::new("0.1.0".to_string()))));
}

#[test]
fn test_157_type_check_wrappers() {
    let source = r#"
ن1 = صحيح؟(42)
ن2 = صحيح؟("نص")
ن3 = عشري؟(3.14)
ن4 = عشري؟(42)
ن5 = نص؟("مرحبا")
ن6 = نص؟(123)
ن7 = قائمة؟([1، 2، 3])
ن8 = قائمة؟("قائمة")
"#;
    let r = run_arabi(source).unwrap();
    assert_eq!(r.globals.get("ن1"), Some(&Value::Boolean(true)));
    assert_eq!(r.globals.get("ن2"), Some(&Value::Boolean(false)));
    assert_eq!(r.globals.get("ن3"), Some(&Value::Boolean(true)));
    assert_eq!(r.globals.get("ن4"), Some(&Value::Boolean(false)));
    assert_eq!(r.globals.get("ن5"), Some(&Value::Boolean(true)));
    assert_eq!(r.globals.get("ن6"), Some(&Value::Boolean(false)));
    assert_eq!(r.globals.get("ن7"), Some(&Value::Boolean(true)));
    assert_eq!(r.globals.get("ن8"), Some(&Value::Boolean(false)));
}
