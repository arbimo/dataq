use itertools::Itertools;
use std::collections::HashMap;

type Symbol = &'static str;

#[derive(Default)]
pub struct Database {
    ids: HashMap<Symbol, u32>,
    interned: Vec<Symbol>,
    db: dataq::Database,
}

impl Database {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn add_fact(&mut self, fact: &[Symbol]) {
        let fact: Vec<dataq::Sym> = fact.iter().copied().map(|s| self.interned(s)).collect();
        self.db.add_fact(&fact)
    }

    fn interned(&mut self, s: &'static str) -> u32 {
        if let Some(id) = self.ids.get(s) {
            *id
        } else {
            let id = self.interned.len() as u32;
            self.interned.push(s);
            self.ids.insert(s, id);
            id
        }
    }

    pub fn run(&self, query: Query) -> impl Iterator<Item = Assignment> + '_ {
        // map each variable name to a unique ID
        let vars: Vec<Symbol> = query.vars().collect();
        let id_of_var: HashMap<Symbol, u32> = vars
            .iter()
            .enumerate()
            .map(|(id, name)| (*name, id as u32))
            .collect();

        let to_core = |atom: Atom| match atom {
            Atom::Sym(s) => dataq::Atom::Sym(self.ids.get(s).copied().unwrap_or(u32::MAX)),
            Atom::Var(v) => dataq::Atom::Var(id_of_var[&v] as dataq::Var),
        };
        let query = query.compile(to_core);

        let state = self.db.run(query);

        state.map(move |ass| {
            let mut bindings = HashMap::with_capacity(ass.len());
            for (id, val) in ass.iter().copied().enumerate() {
                let var = vars[id];
                let val = self.interned[val as usize];
                bindings.insert(var, val);
            }
            bindings
        })
    }
}

/// Maps a variable name to its value
type Assignment = HashMap<Symbol, Symbol>;

#[derive(Copy, Clone)]
enum Atom {
    Sym(Symbol),
    Var(Symbol),
}

impl From<&'static str> for Atom {
    fn from(s: &'static str) -> Self {
        if s.starts_with("?") {
            Atom::Var(s)
        } else {
            Atom::Sym(s)
        }
    }
}

pub struct Query {
    elems: Vec<Vec<Atom>>,
}

impl Query {
    fn single<T: Into<Atom> + Clone>(elem: &[T]) -> Self {
        Query::from(vec![Vec::from(elem)])
    }
    fn from<T: Into<Atom>>(facts: Vec<Vec<T>>) -> Self {
        let mut elems = Vec::with_capacity(facts.len());
        for mut fact in facts {
            let fact = fact.drain(..).map(|t| t.into()).collect();
            elems.push(fact);
        }
        Query { elems }
    }

    pub fn vars(&self) -> impl Iterator<Item = Symbol> + '_ {
        self.elems
            .iter()
            .flat_map(|lf| {
                lf.iter().filter_map(|atom| match atom {
                    Atom::Sym(_) => None,
                    Atom::Var(v) => Some(*v),
                })
            })
            .unique()
    }

    fn compile(self, id: impl Fn(Atom) -> dataq::Atom) -> dataq::Query {
        let mut elems = Vec::with_capacity(self.elems.len());
        for fact in self.elems {
            let fact = fact.iter().map(|atom| id(*atom)).collect();
            elems.push(fact);
        }
        dataq::Query::from(elems)
    }
}

fn main() {
    let mut db = Database::new();
    db.add_fact(&["on", "table", "cup1"]);
    db.add_fact(&["on", "table", "cup2"]);
    db.add_fact(&["in", "kitchen", "table"]);
    db.add_fact(&["in", "bedroom", "bed"]);
    db.add_fact(&["in", "bedroom", "nightstand"]);
    db.add_fact(&["on", "nightstand", "light"]);

    let run_query = |query| {
        for ass in db.run(query) {
            println!("  {:?}", ass);
        }
    };
    let run = |conjunct| {
        println!("query: {:?}", conjunct);
        run_query(Query::single(conjunct));
    };
    let run2 = |c1: &[Symbol], c2: &[Symbol]| {
        println!("query: {:?}  &&  {:?}", c1, c2);
        let q = Query::from(vec![Vec::from(c1), Vec::from(c2)]);
        run_query(q)
    };

    run(&["in", "bedroom", "?x"]);
    run(&["on", "table", "?x"]);
    run(&["on", "?support", "cup1"]);

    run2(&["on", "?support", "cup1"], &["in", "?room", "?support"])
}
