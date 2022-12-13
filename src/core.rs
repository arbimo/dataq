pub type Sym = u32;

type Tuple<E, const N: usize> = [E; 3];

type Fact<const N: usize> = Tuple<Sym, N>;
type FactID = usize;

/// A set of fact with uniform length `N`
#[derive(Default)]
struct Db<const N: usize> {
    facts: Vec<Fact<N>>,
}

impl<const N: usize> Db<N> {
    pub fn add_fact(&mut self, f: Fact<N>) {
        self.facts.push(f)
    }
    pub fn add_fact_n(&mut self, f: &[Sym]) {
        assert_eq!(f.len(), N);
        self.add_fact(f.try_into().unwrap())
    }

    pub fn next_match(&self, pattern: &Pattern, next_index: FactID) -> Option<(FactID, &[Sym])> {
        for (offset, fact) in self.facts[next_index..].iter().enumerate() {
            if pattern.matches(fact) {
                return Some((next_index + offset, fact));
            }
        }
        None
    }
}

/// A set of facts.
///
/// Facts are grouped together organized based on their length.
#[derive(Default)]
pub struct Database {
    db1: Db<1>,
    db2: Db<2>,
    db3: Db<3>,
    db4: Db<4>,
    db5: Db<5>,
    db6: Db<6>,
}

impl Database {
    pub fn new() -> Database {
        Database::default()
    }

    pub fn add_fact(&mut self, f: &[Sym]) {
        match f.len() {
            1 => self.db1.add_fact_n(f),
            2 => self.db2.add_fact_n(f),
            3 => self.db3.add_fact_n(f),
            4 => self.db4.add_fact_n(f),
            5 => self.db5.add_fact_n(f),
            6 => self.db6.add_fact_n(f),
            _ => panic!("Unsupported number of elements in fact"),
        }
    }

    pub fn next_match(&self, pattern: &Pattern, next_index: FactID) -> Option<(FactID, &[Sym])> {
        match pattern.0.len() {
            1 => self.db1.next_match(pattern, next_index),
            2 => self.db2.next_match(pattern, next_index),
            3 => self.db3.next_match(pattern, next_index),
            4 => self.db4.next_match(pattern, next_index),
            5 => self.db5.next_match(pattern, next_index),
            6 => self.db6.next_match(pattern, next_index),
            _ => panic!("Unsupported number of elements in pattern"),
        }
    }

    pub fn run(&self, query: Query) -> impl Iterator<Item = Assignment> + '_ {
        QueryState::new(query, self)
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

    fn matches(&self, fact: &[Sym]) -> bool {
        self.0
            .iter()
            .zip(fact.iter())
            .all(|(pat, sym)| pat.matches(*sym))
    }
}

use itertools::*;

pub type Var = u16;

#[derive(Copy, Clone)]
pub enum Atom {
    Var(Var),
    Sym(Sym),
}

struct LiftedFact(Vec<Atom>);

impl LiftedFact {
    pub fn vars(&self) -> impl Iterator<Item = Var> + '_ {
        self.0
            .iter()
            .filter_map(|atom| match atom {
                Atom::Var(v) => Some(*v),
                _ => None,
            })
            .unique()
    }

    pub fn atoms(&self) -> &[Atom] {
        &self.0
    }
}

pub struct Query {
    elems: Vec<LiftedFact>,
}

impl Query {
    pub fn single(elem: &[Atom]) -> Self {
        Query::from(vec![Vec::from(elem)])
    }
    pub fn from(facts: Vec<Vec<Atom>>) -> Self {
        Query {
            elems: facts.iter().cloned().map(LiftedFact).collect(),
        }
    }
    pub fn vars(&self) -> impl Iterator<Item = Var> + '_ {
        self.elems.iter().flat_map(|lf| lf.vars()).unique()
    }

    pub fn num_vars(&self) -> usize {
        match self.vars().max() {
            Some(max) => (max as usize) + 1,
            None => 0,
        }
    }
}

/// Associated each variable id with its value in the assignment.
pub type Assignment = Vec<Sym>;

/// State of a query being executed.
struct QueryState<'db> {
    database: &'db Database,
    query: Query,
    fact_support: Vec<usize>,
    assignment: Vec<PatternAtom>,
    next_unsupported_fact: usize,
    trail: Vec<(usize, Var)>,
}

impl<'db> QueryState<'db> {
    pub fn new(query: Query, database: &'db Database) -> QueryState {
        let num_vars = query.num_vars();
        let num_patterns = query.elems.len();
        QueryState {
            database,
            query,
            fact_support: (0..num_patterns).map(|_| 0).collect(),
            assignment: (0..num_vars).map(|_| PatternAtom::Wildcard).collect(),
            next_unsupported_fact: 0,
            trail: vec![],
        }
    }

    pub fn undo_last(&mut self) {
        // for the previous fact, undo support and point to the next candidate
        self.next_unsupported_fact -= 1;
        self.fact_support[self.next_unsupported_fact] += 1;
        loop {
            if let Some(&(fact_id, var)) = self.trail.last() {
                if fact_id == self.next_unsupported_fact {
                    self.assignment[var as usize] = PatternAtom::Wildcard;
                    self.trail.pop();
                } else {
                    break;
                }
            } else {
                break;
            }
        }
    }

    pub fn next(&mut self) -> Option<Assignment> {
        if self.next_unsupported_fact == self.fact_support.len() {
            // at a solution, undo it
            self.undo_last()
        } else {
            // must be at init
            assert_eq!(self.next_unsupported_fact, 0);
        }
        while self.next_unsupported_fact < self.fact_support.len() {
            // we have at least a fact that is not supported
            let unusupported_fact = &self.query.elems[self.next_unsupported_fact];
            // build the pattern
            let pattern = Pattern::new(
                unusupported_fact
                    .atoms()
                    .iter()
                    .map(|atom| match atom {
                        Atom::Sym(s) => PatternAtom::Sym(*s),
                        Atom::Var(v) => self.assignment[*v as usize],
                    })
                    .collect(),
            );
            if let Some((support, fact)) = self
                .database
                .next_match(&pattern, self.fact_support[self.next_unsupported_fact])
            {
                for (i, atom) in unusupported_fact.atoms().iter().enumerate() {
                    if let Atom::Var(v) = atom {
                        if self.assignment[*v as usize] == PatternAtom::Wildcard {
                            self.assignment[*v as usize] = PatternAtom::Sym(fact[i]);
                            self.trail.push((self.next_unsupported_fact, *v))
                        }
                    }
                }

                self.fact_support[self.next_unsupported_fact] = support;
                self.next_unsupported_fact += 1;
            } else {
                // no support for this fact, backtrack
                self.fact_support[self.next_unsupported_fact] = 0;
                if self.next_unsupported_fact == 0 {
                    // nothing to backtrack from
                    return None;
                }
                self.undo_last();
            }
        }

        let mut assignment = Vec::with_capacity(self.assignment.len());
        for atom in &self.assignment {
            match atom {
                PatternAtom::Wildcard => {
                    panic!("Malformed query (some variables ids are not used) .")
                }
                PatternAtom::Sym(s) => assignment.push(*s),
            }
        }

        Some(assignment)
    }
}

impl Iterator for QueryState<'_> {
    type Item = Assignment;

    fn next(&mut self) -> Option<Self::Item> {
        self.next()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    pub fn database() -> Database {
        let mut db = Database::new();
        db.add_fact(&[1, 2, 1]);
        db.add_fact(&[1, 2, 2]);
        db.add_fact(&[1, 2, 3]);
        db.add_fact(&[1, 2, 4]);
        db.add_fact(&[1, 2, 5]);

        db.add_fact(&[2, 2, 1]);
        db.add_fact(&[2, 2, 2]);
        db.add_fact(&[2, 2, 3]);
        db.add_fact(&[2, 2, 4]);
        db.add_fact(&[2, 2, 5]);
        db.add_fact(&[2, 2, 6]);
        db.add_fact(&[2, 2, 7]);

        db.add_fact(&[1, 3, 1]);
        db.add_fact(&[1, 3, 2]);
        db.add_fact(&[1, 3, 3]);
        db.add_fact(&[1, 3, 4]);
        db.add_fact(&[1, 3, 5]);
        db.add_fact(&[1, 3, 6]);

        db
    }

    #[test]
    fn test_queries() {
        let db = database();

        let query = Query::single(&[Atom::Sym(1), Atom::Sym(2), Atom::Var(0)]);
        let mut assignments = db.run(query);
        assert_eq!(assignments.next(), Some(vec![1])); // var(0) => 1   (first assignment)
        assert_eq!(assignments.next(), Some(vec![2])); // var(0) => 2   (second assignment)
        assert_eq!(assignments.next(), Some(vec![3])); // ...
        assert_eq!(assignments.next(), Some(vec![4]));
        assert_eq!(assignments.next(), Some(vec![5]));
        assert_eq!(assignments.next(), None); // no assingmnent left

        let query = Query::single(&[Atom::Var(0), Atom::Var(1), Atom::Sym(6)]);
        let mut assignments = db.run(query);
        assert_eq!(assignments.next(), Some(vec![2, 2])); // (var(0) => 2, var(1) => 2 (first assignment)
        assert_eq!(assignments.next(), Some(vec![1, 3])); // (var(0) => 1, var(1) => 3 (second assignment)
        assert_eq!(assignments.next(), None); // no assignment left

        let mut assignments = db.run(Query::single(&[Atom::Sym(1), Atom::Var(0), Atom::Sym(3)]));
        assert_eq!(assignments.next(), Some(vec![2]));
        assert_eq!(assignments.next(), Some(vec![3]));
        assert_eq!(assignments.next(), None);

        let query = Query::single(&[Atom::Sym(1), Atom::Var(0), Atom::Sym(3)]);
        let mut assignments = db.run(query);
        assert_eq!(assignments.next(), Some(vec![2]));
        assert_eq!(assignments.next(), Some(vec![3]));
        assert_eq!(assignments.next(), None);

        let query = Query::from(vec![
            vec![Atom::Var(0), Atom::Var(1), Atom::Sym(3)],
            vec![Atom::Var(0), Atom::Var(2), Atom::Sym(7)],
        ]);
        let mut assignments = db.run(query);
        assert_eq!(assignments.next(), Some(vec![2, 2, 2]));
        assert_eq!(assignments.next(), None);

        let query = Query::from(vec![
            vec![Atom::Var(0), Atom::Var(2), Atom::Sym(7)],
            vec![Atom::Var(0), Atom::Var(1), Atom::Sym(3)],
        ]);
        let mut assignments = db.run(query);
        assert_eq!(assignments.next(), Some(vec![2, 2, 2]));
        assert_eq!(assignments.next(), None);

        let query = Query::from(vec![
            vec![Atom::Var(2), Atom::Var(1), Atom::Sym(7)],
            vec![Atom::Var(2), Atom::Var(0), Atom::Sym(3)],
        ]);
        let mut assignments = db.run(query);
        assert_eq!(assignments.next(), Some(vec![2, 2, 2]));
        assert_eq!(assignments.next(), None);
    }
}
