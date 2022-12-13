use crate::database::*;
use itertools::*;

pub type Var = u16;

#[derive(Copy, Clone)]
pub enum Atom {
    Var(Var),
    Sym(Sym),
}

struct LiftedFact([Atom; 3]);

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

    pub fn atoms(&self) -> &[Atom; 3] {
        &self.0
    }
}

pub struct Query {
    elems: Vec<LiftedFact>,
}

impl Query {
    pub fn single(elem: Tuple<Atom>) -> Self {
        Query::from(vec![elem])
    }
    pub fn from(facts: Vec<Tuple<Atom>>) -> Self {
        Query {
            elems: facts.iter().cloned().map(|v| LiftedFact(v)).collect(),
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

type Assignment = Vec<Sym>;

pub struct State<'db> {
    database: &'db Database,
    query: Query,
    fact_support: Vec<usize>,
    assignment: Vec<PatternAtom>,
    next_unsupported_fact: usize,
    trail: Vec<(usize, Var)>,
}

impl<'db> State<'db> {
    pub fn new(query: Query, database: &'db Database) -> State {
        let num_vars = query.num_vars();
        let num_patterns = query.elems.len();
        State {
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

#[cfg(test)]
mod test {

    use crate::query::{Atom, Query};

    #[test]
    fn test_query() {
        let db = crate::database::test::database();

        let query = Query::single([Atom::Sym(1), Atom::Sym(2), Atom::Var(0)]);
        let mut state = db.run(query);
        assert_eq!(state.next(), Some(vec![1]));
        assert_eq!(state.next(), Some(vec![2]));
        assert_eq!(state.next(), Some(vec![3]));
        assert_eq!(state.next(), Some(vec![4]));
        assert_eq!(state.next(), Some(vec![5]));
        assert_eq!(state.next(), None);

        let query = Query::single([Atom::Var(0), Atom::Var(1), Atom::Sym(6)]);
        let mut state = db.run(query);
        assert_eq!(state.next(), Some(vec![2, 2]));
        assert_eq!(state.next(), Some(vec![1, 3]));
        assert_eq!(state.next(), None);

        let mut state = db.run(Query::single([Atom::Sym(1), Atom::Var(0), Atom::Sym(3)]));
        assert_eq!(state.next(), Some(vec![2]));
        assert_eq!(state.next(), Some(vec![3]));
        assert_eq!(state.next(), None);

        let query = Query::single([Atom::Sym(1), Atom::Var(0), Atom::Sym(3)]);
        let mut state = db.run(query);
        assert_eq!(state.next(), Some(vec![2]));
        assert_eq!(state.next(), Some(vec![3]));
        assert_eq!(state.next(), None);

        let query = Query::from(vec![
            [Atom::Var(0), Atom::Var(1), Atom::Sym(3)],
            [Atom::Var(0), Atom::Var(2), Atom::Sym(7)],
        ]);
        let mut state = db.run(query);
        assert_eq!(state.next(), Some(vec![2, 2, 2]));
        assert_eq!(state.next(), None);

        let query = Query::from(vec![
            [Atom::Var(0), Atom::Var(2), Atom::Sym(7)],
            [Atom::Var(0), Atom::Var(1), Atom::Sym(3)],
        ]);
        let mut state = db.run(query);
        assert_eq!(state.next(), Some(vec![2, 2, 2]));
        assert_eq!(state.next(), None);

        let query = Query::from(vec![
            [Atom::Var(2), Atom::Var(1), Atom::Sym(7)],
            [Atom::Var(2), Atom::Var(0), Atom::Sym(3)],
        ]);
        let mut state = db.run(query);
        assert_eq!(state.next(), Some(vec![2, 2, 2]));
        assert_eq!(state.next(), None);
    }
}
