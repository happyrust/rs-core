use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use std::sync::OnceLock;
use tokio::runtime::Runtime;

use aios_core::{RefnoEnum, SUL_DB, SurrealQueryExt};

struct PeTreeFixture {
    dbnum: u32,
    branching: usize,
    depth: usize,
}

impl PeTreeFixture {
    fn node_count(&self) -> usize {
        // 1 + b + b^2 + ... + b^depth
        let mut total = 0usize;
        let mut p = 1usize;
        for _ in 0..=self.depth {
            total += p;
            p *= self.branching;
        }
        total
    }

    fn build_create_sql(&self) -> String {
        let mut levels: Vec<Vec<usize>> = Vec::with_capacity(self.depth + 1);
        let mut next_id: usize = 1;
        levels.push(vec![next_id]);
        next_id += 1;

        for _ in 0..self.depth {
            let prev = levels.last().expect("levels must not be empty");
            let mut cur: Vec<usize> = Vec::with_capacity(prev.len() * self.branching);
            for _ in prev {
                for _ in 0..self.branching {
                    cur.push(next_id);
                    next_id += 1;
                }
            }
            levels.push(cur);
        }

        let mut sql = String::with_capacity(self.node_count() * 80);
        for (level_index, nodes) in levels.iter().enumerate() {
            for (node_index, node_id) in nodes.iter().copied().enumerate() {
                let noun = if level_index == 0 { "SITE" } else { "EQUI" };
                let children_ids: &[usize] = if level_index >= self.depth {
                    &[]
                } else {
                    let start = node_index * self.branching;
                    let end = start + self.branching;
                    &levels[level_index + 1][start..end]
                };

                let rid = format!("pe:⟨{}_{}⟩", self.dbnum, node_id);
                if children_ids.is_empty() {
                    sql.push_str(&format!(
                        "CREATE {} SET noun='{}', deleted=false, children=[];\n",
                        rid, noun
                    ));
                    continue;
                }

                let mut child_rids = String::with_capacity(children_ids.len() * 16);
                for (i, child_id) in children_ids.iter().copied().enumerate() {
                    if i > 0 {
                        child_rids.push_str(", ");
                    }
                    child_rids.push_str(&format!("pe:⟨{}_{}⟩", self.dbnum, child_id));
                }

                sql.push_str(&format!(
                    "CREATE {} SET noun='{}', deleted=false, children=[{}];\n",
                    rid, noun, child_rids
                ));
            }
        }

        sql
    }

    fn roots_at_level1(&self, count: usize) -> Vec<RefnoEnum> {
        // level1 的节点 id 从 2 开始，连续 branching 个
        let max = count.min(self.branching);
        (0..max)
            .map(|i| RefnoEnum::from(format!("{}_{}", self.dbnum, 2 + i).as_str()))
            .collect()
    }
}

fn setup(rt: &Runtime) {
    static INIT: OnceLock<()> = OnceLock::new();
    INIT.get_or_init(|| {
        rt.block_on(async {
            // 1) 连接嵌入式内存 DB（避免依赖外部 SurrealDB 服务）
            if let Err(e) = SUL_DB.connect("mem://").await {
                // bench 可能在同进程内被多次初始化，这里允许 Already connected
                if !e.to_string().contains("Already connected") {
                    panic!("connect mem:// failed: {e}");
                }
            }

            // 2) 设置 NS/DB
            SUL_DB
                .use_ns("bench")
                .use_db("bench")
                .await
                .expect("use ns/db failed");

            // 3) 加载 resource/surreal/*.surql（提供 fn::collect_descendant_ids_by_types 等函数）
            aios_core::function::define_common_functions(Some("resource/surreal"))
                .await
                .expect("define_common_functions failed");

            // 4) 构造一棵小型 pe 树（通过 children 字段递归）
            let fixture = PeTreeFixture {
                dbnum: 1,
                branching: 5,
                depth: 4,
            };
            let create_sql = fixture.build_create_sql();
            let _ = SUL_DB
                .query_response(&create_sql)
                .await
                .expect("create fixture failed");

            // 5) 快速校验：查询子孙（过滤掉 root SITE，只取 EQUI）
            let root: RefnoEnum = "1_1".into();
            let ids = aios_core::collect_descendant_ids_batch(&[root], &["EQUI"], None)
                .await
                .expect("collect_descendant_ids_batch failed");

            let expected = fixture.node_count() - 1;
            assert_eq!(ids.len(), expected, "fixture descendants mismatch");
        });
    });
}

fn bench_collect_descendant_ids_batch(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    setup(&rt);

    let fixture = PeTreeFixture {
        dbnum: 1,
        branching: 5,
        depth: 4,
    };

    let mut group = c.benchmark_group("collect_descendant_ids_batch");
    for root_count in [1usize, 2, 5] {
        let roots = fixture.roots_at_level1(root_count);
        group.bench_with_input(BenchmarkId::new("roots", root_count), &roots, |b, roots| {
            b.to_async(&rt).iter(|| async {
                let ids = aios_core::collect_descendant_ids_batch(
                    black_box(roots.as_slice()),
                    black_box(&["EQUI"]),
                    black_box(None::<&str>),
                )
                .await
                .expect("query failed");
                black_box(ids);
            });
        });
    }
    group.finish();
}

criterion_group!(benches, bench_collect_descendant_ids_batch);
criterion_main!(benches);
