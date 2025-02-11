use std::collections::HashMap;
use std::collections::BTreeSet;
use serde::{Serialize};
use crate::fiero::{Fiero,to_syllabics};

#[derive(Clone, PartialOrd, Ord, PartialEq, Eq, Serialize)]
pub struct OJWord {
    pub word: String,
    pub suffix: Option<String>,
    pub meta: Option<String>,
}

#[derive(PartialOrd, Ord, PartialEq, Eq, Serialize)]
pub struct DictEntry {
    #[serde(skip)]
    pub fiero: Vec<Fiero>,
    pub syllabics: String,
    pub oj: OJWord,
    pub en: Vec<String>,
}

impl OJWord {
    fn parse(raw_word: &str, raw_meta: &str) -> Self{
        let mut iter = raw_word.split('+');
        let word = String::from(iter.next().unwrap());
        let suffix = iter.next().map(String::from);
        let meta = if raw_meta.is_empty() {None} else {Some(String::from(raw_meta))};
        OJWord{word, suffix, meta}
    }

    pub fn contains(&self, query: &str) -> bool {
        match &self.suffix {
            None => self.word.contains(query),
            Some(pl) => self.word.contains(query) || (self.word.clone() + &pl).contains(query),
        }
    }

    fn rm(&self) -> String {
        match &self.suffix {
            None => self.word.clone(),
            Some(pl) => self.word.clone() + "+" + &pl,
        }
    }
}

impl DictEntry {
    pub fn en_contains(&self, query: &str) -> bool {
        self.en.iter().any(|s| {s.to_lowercase().contains(query)})
    }

    pub fn as_tsv(&self) -> String {
        let mut line = format!("{}\t{}\t{}",
            self.oj.meta.as_deref().unwrap_or(""),
            self.oj.rm(),
            self.syllabics);
        for def in self.en.iter() {
            line += "\t";
            line += def;
        }
        line
    }
}

fn unrotate(input: &str) -> Option<String> {

    if let [word, ctx] = input.split(": ").collect::<Vec<_>>()[..] {
        if let [before, after] = ctx.split("~").collect::<Vec<_>>()[..] {
            return Some(format!("{}{}{}", before, word, after));
        }
    }
    None
}

fn make_entry(((oj, meta), en): ((&str, &str), Vec<&str>)) -> DictEntry {
    let oj = OJWord::parse(oj, meta);
    let fiero = Fiero::parse(&oj.word);
    let mut defs: BTreeSet<String> = BTreeSet::new();
    let mut rots: Vec<(&str, String)> = Vec::new();

    // take clearly unrotated strings
    for def in en {
        match unrotate(def) {
            Some(s) => rots.push((def,s)),
            None => {defs.insert(String::from(def));}
        }
    }
    // filter potential rotations between duplicates and unique terms.
    for (rot, unrot) in rots.into_iter() {
        if !defs.contains(&unrot) {
            defs.insert(String::from(rot));
        }
    }
    let defs_vec = defs.into_iter().collect();
    let syllabics = to_syllabics(&fiero);
    DictEntry {fiero, syllabics, oj, en: defs_vec}
}

// parse TSV and collect duplicate entries
pub fn parse_dict(raw_dict: String) -> Vec<DictEntry> {
    let mut entries: HashMap<(&str, &str), Vec<&str>> = HashMap::new();

    for line in raw_dict.lines() {
        let fields: Vec<&str> = line.split('\t').collect();
        if let [meta, oj, en] = fields[..] {
            entries.entry((oj, meta)).or_insert(Vec::with_capacity(1)).push(en);
        }
    }

    let mut dict: Vec<DictEntry> = entries.into_iter().map(make_entry).collect();
    drop(raw_dict);
    dict.sort_unstable();
    dict
}

fn order_by_meta(a: &&DictEntry, b: &&DictEntry) -> std::cmp::Ordering {
    (&a.oj.meta,&a.fiero,a).cmp(&(&b.oj.meta,&b.fiero,b))
}

pub fn sorted_by_meta(dict: &Vec<DictEntry>) -> Vec<&DictEntry> {
    let mut grouped: Vec<&DictEntry> = dict.iter().collect();
    grouped.sort_by(&order_by_meta);
    grouped
}

pub fn dict_to_tsv(dict: &Vec<DictEntry>) -> String {
    let mut out = String::new();
    let grouped = sorted_by_meta(dict);
    for entry in grouped {
        out += &entry.as_tsv();
        out += "\n";
    }
    out
}