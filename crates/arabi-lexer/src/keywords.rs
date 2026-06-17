use std::collections::HashMap;
use arabi_core::token::Keyword;

pub struct KeywordMap {
    map: HashMap<&'static str, Keyword>,
}

impl Default for KeywordMap {
    fn default() -> Self {
        Self::new()
    }
}

impl KeywordMap {
    pub fn new() -> Self {
        let mut map = HashMap::new();

        // Control flow
        map.insert("اذا", Keyword::If);
        map.insert("اواذا", Keyword::Elif);
        map.insert("والا", Keyword::Else);
        map.insert("الا", Keyword::Else);
        map.insert("بينما", Keyword::While);
        map.insert("لكل", Keyword::For);
        map.insert("في", Keyword::In);
        map.insert("توقف", Keyword::Break);
        map.insert("استمر", Keyword::Continue);
        map.insert("مرور", Keyword::Pass);

        // Functions
        map.insert("دالة", Keyword::Function);
        map.insert("ارجع", Keyword::Return);
        map.insert("خطية", Keyword::Lambda);

        // Classes
        map.insert("صنف", Keyword::Class);
        map.insert("هذا", Keyword::Self_);
        map.insert("اصل", Keyword::Super);

        // Import
        map.insert("استورد", Keyword::Import);
        map.insert("من", Keyword::From);
        map.insert("بشرط", Keyword::As);

        // Exception handling
        map.insert("حاول", Keyword::Try);
        map.insert("خلل", Keyword::Except);
        map.insert("نهاية", Keyword::Finally);
        map.insert("ارم", Keyword::Raise);

        // Other
        map.insert("احذف", Keyword::Delete);
        map.insert("اكد", Keyword::Assert);
        map.insert("يساوي", Keyword::Is);
        map.insert("عام", Keyword::Global);
        map.insert("محلي", Keyword::Nonlocal);
        map.insert("و", Keyword::And);
        map.insert("او", Keyword::Or);
        map.insert("ليس", Keyword::Not);
        map.insert("صح", Keyword::True);
        map.insert("خطا", Keyword::False);
        map.insert("عدم", Keyword::None);

        // Generators
        map.insert("سلم", Keyword::Yield);
        map.insert("سلم_من", Keyword::YieldFrom);

        // Context managers
        map.insert("باستخدام", Keyword::With);

        // Decorators
        map.insert("زخرف", Keyword::Decorator);

        // Match/Case
        map.insert("طابق", Keyword::Match);
        map.insert("حالة", Keyword::Case);
        map.insert("حالة_اخرى", Keyword::CaseDefault);

        KeywordMap { map }
    }

    pub fn lookup(&self, word: &str) -> Option<&Keyword> {
        self.map.get(word)
    }
}
