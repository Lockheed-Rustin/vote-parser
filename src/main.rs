use prettytable::{row, Table};
use serde::Deserialize;
use std::{
    collections::{HashMap, HashSet},
    fs::{self, File},
    io::BufWriter,
};

const BOUGHT_COUNT: u32 = 10;

#[derive(Deserialize, Clone, Debug)]
struct Groups {
    group: Vec<Group>,
}

#[derive(Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
struct Group {
    name: String,
    aliases: Option<Vec<String>>,
}

impl Group {
    fn similarity(&self, other: &str) -> f64 {
        strsim::normalized_damerau_levenshtein(&self.name, other).max(
            self.aliases
                .iter()
                .flatten()
                .map(|alias| strsim::normalized_damerau_levenshtein(alias, other))
                .max_by(|a, b| a.partial_cmp(b).unwrap())
                .unwrap_or_default(),
        )
    }
}

fn parse_group_name(vote: &str) -> String {
    // hopefully nobody's name start with a non alphabetic char
    vote.trim_start_matches(|c: char| !c.is_ascii_alphabetic())
        .to_string()
}

fn parse_votes(groups: &Groups, votes: &str) -> HashMap<Group, (u32, HashSet<String>)> {
    let mut result = HashMap::new();

    for vote_message in votes.split("> ").skip(1) {
        let lines = vote_message.trim().lines().skip(2);
        assert_eq!(lines.clone().count(), BOUGHT_COUNT as usize);
        for line in lines {
            let aliased = parse_group_name(line);

            let group = groups
                .group
                .iter()
                .max_by(|a, b| {
                    a.similarity(&aliased)
                        .partial_cmp(&b.similarity(&aliased))
                        .unwrap()
                })
                .unwrap()
                .clone();
            let entry: &mut (u32, HashSet<String>) = result.entry(group).or_default();
            entry.0 += 1;
            entry.1.insert(aliased);
        }
    }

    result
}

fn print_result(votes: &HashMap<Group, (u32, HashSet<String>)>) {
    let file = File::create("votes.md").unwrap();
    let mut buff = BufWriter::new(file);

    let mut votes = votes.iter().collect::<Vec<_>>();
    votes.sort_by(|a, b| b.1 .0.cmp(&a.1 .0));

    let mut table = Table::new();
    let votes_count = votes
        .iter()
        .map(|(_, (vote_count, _))| *vote_count)
        .sum::<u32>()
        / BOUGHT_COUNT;
    let yet_to_vote = votes.len() as u32 - votes_count;
    table.add_row(row!["VOTES COUNT", "YET TO VOTE"]);
    table.add_row(row![votes_count, yet_to_vote]);
    table.print_html(&mut buff).unwrap();

    let mut table = Table::new();
    table.add_row(row!["GROUP NAME", "ALIASES", "VOTE COUNT", "ALIASED"]);
    for (group, (vote_count, aliased)) in votes.iter() {
        let aliased = aliased.iter().cloned().collect::<Vec<_>>().join(", ");
        let aliases = group
            .aliases
            .iter()
            .flatten()
            .cloned()
            .collect::<Vec<_>>()
            .join(", ");
        table.add_row(row![group.name, aliases, vote_count, aliased]);
    }

    table.print_html(&mut buff).unwrap();
    table.printstd();
}

fn main() {
    let groups = fs::read_to_string("groups.toml").unwrap();
    let groups: Groups = toml::from_str(&groups).unwrap();
    let votes = fs::read_to_string("votes.txt").unwrap();

    let parsed_votes = parse_votes(&groups, &votes);

    print_result(&parsed_votes);
}
