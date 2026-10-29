//! Replay the baseline navigation corpus and spatial samples against offline graphs.
use glam::Vec3;
use navigation::{NavigationMap, read_walkable_polygons};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::Path;
use std::time::Instant;

fn spatial_sample(polygons: Vec<Vec<[f32; 3]>>, count: usize) -> Vec<Vec3> {
    let mut centers: Vec<Vec3> = polygons
        .into_iter()
        .map(|poly| {
            let n = poly.len() as f32;
            poly.into_iter().map(Vec3::from_array).sum::<Vec3>() / n
        })
        .collect();
    centers.sort_by(|a, b| {
        a.x.total_cmp(&b.x)
            .then(a.z.total_cmp(&b.z))
            .then(a.y.total_cmp(&b.y))
    });
    if centers.is_empty() {
        return Vec::new();
    }
    let mut distances = vec![f32::INFINITY; centers.len()];
    let mut picked = Vec::new();
    let mut index = 0;
    for _ in 0..count.min(centers.len()) {
        let point = centers[index];
        picked.push(point);
        for (d, p) in distances.iter_mut().zip(&centers) {
            *d = d.min(p.distance_squared(point));
        }
        let Some((next, &best)) = distances
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.total_cmp(b.1))
        else {
            break;
        };
        if best <= 0.0 {
            break;
        }
        index = next;
    }
    picked
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 4 {
        return Err("usage: hierarchy_sweep GRAPH_ROOT CORPUS_TSV BASELINE_TSV".into());
    }
    let root = Path::new(&args[1]);
    let corpus = std::fs::read_to_string(&args[2])?;
    let mut maps: BTreeMap<u32, Vec<(String, Vec3)>> = BTreeMap::new();
    for line in corpus.lines() {
        let f: Vec<_> = line.split('\t').collect();
        maps.entry(f[1].parse()?).or_default().push((
            f[2].to_owned(),
            Vec3::new(f[3].parse()?, f[4].parse()?, f[5].parse()?),
        ));
    }
    let baseline = std::fs::read_to_string(&args[3])?;
    let connectivity = std::env::var_os("HIERARCHY_CONNECTIVITY").is_some();
    let mut wanted = HashSet::new();
    for line in baseline.lines().skip(1) {
        let f: Vec<_> = line.split('\t').collect();
        if (!connectivity
            && (f[7] == "complete"
                || f[7] == "partial"
                || (std::env::var_os("HIERARCHY_ORACLE_PAIRS").is_some() && f[7] == "connected")))
            || (connectivity
                && (f[7] == "unreachable target (hierarchy disconnected)"
                    || f[7] == "unreachable target (incomplete corridor)"))
        {
            wanted.insert((
                f[0].to_owned(),
                f[1].to_owned(),
                f[2].to_owned(),
                f[3].to_owned(),
            ));
        }
    }
    println!("map\tkind\tfrom\tto\tlegs\ttotal_us\tmax_leg_us\tresult");
    for (id, corpus) in maps {
        let dir = root.join(id.to_string());
        let mut nav = NavigationMap::open(&dir)?;
        let polygons = read_walkable_polygons(&dir)?;
        let samples = spatial_sample(polygons, 50);
        eprintln!("starting {id}");
        for kind in ["corpus", "sample"] {
            let points: HashMap<String, Vec3> = if kind == "corpus" {
                corpus.clone().into_iter().collect()
            } else {
                samples
                    .iter()
                    .enumerate()
                    .map(|(i, p)| (i.to_string(), *p))
                    .collect()
            };
            let mut pairs: Vec<_> = wanted
                .iter()
                .filter(|(map, k, _, _)| map == &id.to_string() && k == kind)
                .collect();
            pairs.sort();
            if connectivity {
                #[cfg(feature = "bake")]
                {
                    let mut grouped: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
                    for (_, _, a, b) in &pairs {
                        grouped.entry(a.as_str()).or_default().push(b.as_str());
                    }
                    for (from, targets) in grouped {
                        let start = *points.get(from).ok_or("missing source sample")?;
                        let destinations = targets
                            .iter()
                            .map(|target| {
                                points.get(*target).copied().ok_or("missing target sample")
                            })
                            .collect::<Result<Vec<_>, _>>()?;
                        let reachable = nav.diagnostic_reachable(start, &destinations)?;
                        for (target, connected) in targets.into_iter().zip(reachable) {
                            println!(
                                "{id}\t{kind}\t{from}\t{target}\t0\t0\t0\t{}",
                                if connected {
                                    "connected"
                                } else {
                                    "disconnected"
                                }
                            );
                        }
                    }
                    continue;
                }
                #[cfg(not(feature = "bake"))]
                return Err("connectivity probe requires --features bake".into());
            }
            for (_, _, a, b) in pairs {
                let (Some(&start), Some(&end)) = (points.get(a), points.get(b)) else {
                    return Err(format!("sample missing {id} {kind} {a} {b}").into());
                };
                let started = Instant::now();
                let mut current = start;
                let mut seen = HashSet::new();
                let mut plan = navigation::NavigationPlan::default();
                let mut legs = 0;
                let mut max_leg = 0;
                let result = loop {
                    let leg_start = Instant::now();
                    let path = nav.path_with_plan(current, end, &mut plan);
                    max_leg = max_leg.max(leg_start.elapsed().as_micros());
                    legs += 1;
                    match path {
                        Ok(path) if path.complete => break "complete".to_owned(),
                        Ok(path) => match path.points.last() {
                            Some(&point)
                                if point != current
                                    && seen.insert(point.to_array().map(f32::to_bits))
                                    && legs < 1024 =>
                            {
                                current = point
                            }
                            _ => break "no progress or leg limit".to_owned(),
                        },
                        Err(error) => break error.replace(['\t', '\n'], " "),
                    }
                };
                println!(
                    "{id}\t{kind}\t{a}\t{b}\t{legs}\t{}\t{max_leg}\t{result}",
                    started.elapsed().as_micros()
                );
            }
        }
        eprintln!("finished {id}");
    }
    Ok(())
}
