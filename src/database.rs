use crate::query::{Query, State};

pub(crate) type Sym = u32;

pub type Tuple<E> = [E; 3];

type Fact = Tuple<Sym>;
type FactID = usize;

#[derive(Default)]
pub struct Database {
    facts: Vec<Fact>,
}

impl Database {
    pub fn new() -> Database {
        Database::default()
    }

    pub fn add_fact(&mut self, f: Fact) {
        self.facts.push(f)
    }

    pub fn next_match(&self, pattern: &Pattern, next_index: FactID) -> Option<(FactID, &Fact)> {
        for (offset, fact) in self.facts[next_index..].iter().enumerate() {
            if pattern.matches(fact) {
                return Some((next_index + offset, fact));
            }
        }
        return None;
    }

    pub fn run(&self, query: Query) -> State {
        State::new(query, self)
    }
}

#[derive(Ord, PartialOrd, Eq, PartialEq, Copy, Clone)]
pub enum PatternAtom {
    Wildcard,
    Sym(Sym),
}
impl PatternAtom {
    fn matches(self, sym: Sym) -> bool {
        match self {
            PatternAtom::Wildcard => true,
            PatternAtom::Sym(s) => s == sym,
        }
    }
}

pub struct Pattern(Vec<PatternAtom>);

impl Pattern {
    pub fn new(elems: Vec<PatternAtom>) -> Self {
        Pattern(elems)
    }

    fn matches(&self, fact: &Fact) -> bool {
        self.0
            .iter()
            .zip(fact.iter())
            .all(|(pat, sym)| pat.matches(*sym))
    }
}

#[cfg(test)]
pub mod test {
    use crate::database::Database;

    pub fn database() -> Database {
        let mut db = Database::new();
        db.add_fact([1, 2, 1]);
        db.add_fact([1, 2, 2]);
        db.add_fact([1, 2, 3]);
        db.add_fact([1, 2, 4]);
        db.add_fact([1, 2, 5]);

        db.add_fact([2, 2, 1]);
        db.add_fact([2, 2, 2]);
        db.add_fact([2, 2, 3]);
        db.add_fact([2, 2, 4]);
        db.add_fact([2, 2, 5]);
        db.add_fact([2, 2, 6]);
        db.add_fact([2, 2, 7]);

        db.add_fact([1, 3, 1]);
        db.add_fact([1, 3, 2]);
        db.add_fact([1, 3, 3]);
        db.add_fact([1, 3, 4]);
        db.add_fact([1, 3, 5]);
        db.add_fact([1, 3, 6]);

        db
    }

    #[test]
    fn test_database_creation() {
        database();
    }
}
