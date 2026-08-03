use crate::pdms_data::PlinParamData;
use crate::{NamedAttrMap, RefnoEnum};
use async_trait::async_trait;
use glam::DVec3;

/// 变换计算所需的只读事实源。
///
/// 策略只依赖该接口，不关心事实来自 SurrealDB、解析暂存库或测试 fixture。
/// 返回的 children/ancestor 顺序必须与 PDMS 层级顺序一致。
#[async_trait]
pub trait TransformFactSource: Send + Sync {
    async fn get_attribute(&self, refno: RefnoEnum) -> anyhow::Result<NamedAttrMap>;

    async fn get_attributes(&self, refnos: &[RefnoEnum]) -> anyhow::Result<Vec<NamedAttrMap>> {
        let mut result = Vec::with_capacity(refnos.len());
        for refno in refnos {
            result.push(self.get_attribute(*refno).await?);
        }
        Ok(result)
    }

    async fn get_owner(&self, refno: RefnoEnum) -> anyhow::Result<RefnoEnum> {
        Ok(self.get_attribute(refno).await?.get_owner())
    }

    async fn get_children(&self, refno: RefnoEnum) -> anyhow::Result<Vec<RefnoEnum>>;

    async fn get_children_attributes(&self, refno: RefnoEnum) -> anyhow::Result<Vec<NamedAttrMap>> {
        let children = self.get_children(refno).await?;
        self.get_attributes(&children).await
    }

    /// 返回 root-to-parent 顺序的祖先链，不包含当前节点。
    async fn get_ancestors(&self, refno: RefnoEnum) -> anyhow::Result<Vec<RefnoEnum>>;

    async fn get_ancestors_of_types(
        &self,
        refno: RefnoEnum,
        nouns: &[&str],
    ) -> anyhow::Result<Vec<RefnoEnum>> {
        let ancestors = self.get_ancestors(refno).await?;
        let attributes = self.get_attributes(&ancestors).await?;
        Ok(attributes
            .into_iter()
            .filter(|attribute| {
                let noun = attribute.get_type_str();
                nouns.iter().any(|expected| *expected == noun)
            })
            .filter_map(|attribute| attribute.get_refno())
            .collect())
    }

    async fn query_pline(
        &self,
        refno: RefnoEnum,
        key: &str,
    ) -> anyhow::Result<Option<PlinParamData>>;

    async fn get_spline_line_dir(&self, refno: RefnoEnum) -> anyhow::Result<DVec3> {
        let children = self.get_children_attributes(refno).await?;
        let direct_points: Vec<DVec3> = children
            .iter()
            .filter(|attribute| attribute.get_type_str() == "POINSP")
            .filter_map(|attribute| attribute.get_position().map(|value| value.as_dvec3()))
            .collect();
        let points = if direct_points.len() == 2 {
            direct_points
        } else {
            let spine = children
                .iter()
                .find(|attribute| attribute.get_type_str() == "SPINE")
                .and_then(NamedAttrMap::get_refno)
                .ok_or_else(|| anyhow::anyhow!("{} 下不存在 SPINE", refno))?;
            self.get_children_attributes(spine)
                .await?
                .into_iter()
                .filter(|attribute| attribute.get_type_str() == "POINSP")
                .filter_map(|attribute| attribute.get_position().map(|value| value.as_dvec3()))
                .collect()
        };
        anyhow::ensure!(points.len() == 2, "{} 下没有恰好两个 SPINE 点", refno);
        Ok((points[1] - points[0]).normalize())
    }
}

/// 旧路径使用的 SurrealDB 事实源。所有兼容入口都委托到该实现。
#[derive(Clone, Copy, Debug, Default)]
pub struct SurrealTransformFactSource;

#[async_trait]
impl TransformFactSource for SurrealTransformFactSource {
    async fn get_attribute(&self, refno: RefnoEnum) -> anyhow::Result<NamedAttrMap> {
        crate::get_named_attmap(refno).await
    }

    async fn get_children(&self, refno: RefnoEnum) -> anyhow::Result<Vec<RefnoEnum>> {
        crate::get_children_refnos(refno).await
    }

    async fn get_children_attributes(&self, refno: RefnoEnum) -> anyhow::Result<Vec<NamedAttrMap>> {
        crate::get_children_named_attmaps(refno).await
    }

    async fn get_ancestors(&self, refno: RefnoEnum) -> anyhow::Result<Vec<RefnoEnum>> {
        crate::query_ancestor_refnos(refno).await
    }

    async fn get_ancestors_of_types(
        &self,
        refno: RefnoEnum,
        nouns: &[&str],
    ) -> anyhow::Result<Vec<RefnoEnum>> {
        crate::query_filter_ancestors(refno, nouns).await
    }

    async fn query_pline(
        &self,
        refno: RefnoEnum,
        key: &str,
    ) -> anyhow::Result<Option<PlinParamData>> {
        crate::query_pline(refno, key.to_string()).await
    }

    async fn get_spline_line_dir(&self, refno: RefnoEnum) -> anyhow::Result<DVec3> {
        crate::rs_surreal::spatial::get_spline_line_dir(refno).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AttrVal;
    use crate::pdms_data::PlinParamData;
    use crate::transform::{
        get_effective_parent_att_with_source, get_local_mat4_with_source,
        get_world_mat4_with_source,
    };
    use async_trait::async_trait;
    use std::collections::HashMap;
    use std::sync::Arc;

    #[derive(Default)]
    struct FixtureTransformFactSource {
        attributes: HashMap<RefnoEnum, NamedAttrMap>,
        children: HashMap<RefnoEnum, Vec<RefnoEnum>>,
    }

    #[async_trait]
    impl TransformFactSource for FixtureTransformFactSource {
        async fn get_attribute(&self, refno: RefnoEnum) -> anyhow::Result<NamedAttrMap> {
            self.attributes
                .get(&refno)
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("missing fixture attribute for {}", refno))
        }

        async fn get_children(&self, refno: RefnoEnum) -> anyhow::Result<Vec<RefnoEnum>> {
            Ok(self.children.get(&refno).cloned().unwrap_or_default())
        }

        async fn get_ancestors(&self, refno: RefnoEnum) -> anyhow::Result<Vec<RefnoEnum>> {
            let mut result = Vec::new();
            let mut current = self.get_owner(refno).await?;
            while !current.is_unset() {
                result.push(current);
                current = self.get_owner(current).await?;
            }
            result.reverse();
            Ok(result)
        }

        async fn query_pline(
            &self,
            _refno: RefnoEnum,
            _key: &str,
        ) -> anyhow::Result<Option<PlinParamData>> {
            Ok(None)
        }
    }

    fn fixture_attribute(
        refno: RefnoEnum,
        owner: RefnoEnum,
        noun: &str,
        position: Option<[f64; 3]>,
    ) -> NamedAttrMap {
        let mut attribute = NamedAttrMap::new(noun);
        attribute.insert(
            "REFNO".to_string(),
            AttrVal::RefU64Type(refno.refno()).into(),
        );
        attribute.insert(
            "OWNER".to_string(),
            AttrVal::RefU64Type(owner.refno()).into(),
        );
        if let Some(position) = position {
            attribute.insert("POS".to_string(), AttrVal::Vec3Type(position).into());
        }
        attribute
    }

    #[tokio::test]
    async fn injected_source_computes_default_local_transform() {
        let parent = RefnoEnum::from("10/1");
        let child = RefnoEnum::from("10/2");
        let mut source = FixtureTransformFactSource::default();
        source.attributes.insert(
            parent,
            fixture_attribute(parent, RefnoEnum::default(), "SITE", None),
        );
        source.attributes.insert(
            child,
            fixture_attribute(child, parent, "EQUI", Some([1.0, 2.0, 3.0])),
        );

        let matrix = get_local_mat4_with_source(child, Arc::new(source))
            .await
            .expect("local transform")
            .expect("matrix");
        assert_eq!(matrix.w_axis.truncate(), DVec3::new(1.0, 2.0, 3.0));
    }

    #[tokio::test]
    async fn root_without_local_transform_uses_identity() {
        let root = RefnoEnum::from("10/1");
        let mut source = FixtureTransformFactSource::default();
        source.attributes.insert(
            root,
            fixture_attribute(root, RefnoEnum::default(), "WORLD", None),
        );

        let matrix = get_local_mat4_with_source(root, Arc::new(source))
            .await
            .expect("root transform")
            .expect("matrix");
        assert_eq!(matrix, glam::DMat4::IDENTITY);
    }

    #[tokio::test]
    async fn virtual_parent_cycle_fails_closed() {
        let first = RefnoEnum::from("10/1");
        let second = RefnoEnum::from("10/2");
        let mut source = FixtureTransformFactSource::default();
        source
            .attributes
            .insert(first, fixture_attribute(first, second, "SPINE", None));
        source
            .attributes
            .insert(second, fixture_attribute(second, first, "SPINE", None));

        let error = get_effective_parent_att_with_source(first, Arc::new(source))
            .await
            .expect_err("cycle must be rejected");
        assert!(error.to_string().contains("Cycle detected"));
    }

    #[tokio::test]
    async fn virtual_parent_attributes_are_merged_from_real_owner() {
        let real_owner = RefnoEnum::from("10/1");
        let virtual_owner = RefnoEnum::from("10/2");
        let mut real = fixture_attribute(real_owner, RefnoEnum::default(), "GENSEC", None);
        real.insert(
            "REAL_MARK".to_string(),
            AttrVal::StringType("real".to_string()).into(),
        );
        let mut virtual_attribute = fixture_attribute(virtual_owner, real_owner, "SPINE", None);
        virtual_attribute.insert(
            "VIRTUAL_MARK".to_string(),
            AttrVal::StringType("virtual".to_string()).into(),
        );
        let mut source = FixtureTransformFactSource::default();
        source.attributes.insert(real_owner, real);
        source.attributes.insert(virtual_owner, virtual_attribute);

        let merged = get_effective_parent_att_with_source(virtual_owner, Arc::new(source))
            .await
            .expect("merged attributes");
        assert_eq!(merged.get_str("REAL_MARK"), Some("real"));
        assert_eq!(merged.get_str("VIRTUAL_MARK"), Some("virtual"));
        assert_eq!(merged.get_type_str(), "SPINE");
    }

    #[tokio::test]
    async fn injected_source_covers_spine_gensec_wall_and_world_composition() {
        let world_root = RefnoEnum::from("10/10");
        let site = RefnoEnum::from("10/1");
        let gensec = RefnoEnum::from("10/2");
        let spine = RefnoEnum::from("10/3");
        let first_point = RefnoEnum::from("10/4");
        let second_point = RefnoEnum::from("10/5");
        let datum = RefnoEnum::from("10/6");
        let wall = RefnoEnum::from("10/7");

        let root_attribute = fixture_attribute(world_root, RefnoEnum::default(), "WORLD", None);
        let mut site_attribute =
            fixture_attribute(site, world_root, "SITE", Some([10.0, 0.0, 0.0]));
        site_attribute.insert(
            "POS".to_string(),
            AttrVal::Vec3Type([10.0, 0.0, 0.0]).into(),
        );
        let gensec_attribute = fixture_attribute(gensec, site, "GENSEC", None);
        let mut spine_attribute = fixture_attribute(spine, gensec, "SPINE", None);
        spine_attribute.insert(
            "YDIR".to_string(),
            AttrVal::Vec3Type([0.0, 1.0, 0.0]).into(),
        );
        let first_attribute =
            fixture_attribute(first_point, spine, "POINSP", Some([0.0, 0.0, 0.0]));
        let second_attribute =
            fixture_attribute(second_point, spine, "POINSP", Some([0.0, 0.0, 10.0]));
        let mut datum_attribute = fixture_attribute(datum, gensec, "JLDATU", None);
        datum_attribute.insert("PKDI".to_string(), AttrVal::DoubleType(0.5).into());
        datum_attribute.insert("ZDIS".to_string(), AttrVal::DoubleType(0.0).into());
        let mut wall_attribute = fixture_attribute(wall, site, "STWALL", Some([0.0, 2.0, 0.0]));
        wall_attribute.insert(
            "DPOSS".to_string(),
            AttrVal::Vec3Type([0.0, 0.0, 0.0]).into(),
        );
        wall_attribute.insert(
            "DPOSE".to_string(),
            AttrVal::Vec3Type([0.0, 0.0, 5.0]).into(),
        );

        let mut source = FixtureTransformFactSource::default();
        source.attributes.extend([
            (world_root, root_attribute),
            (site, site_attribute),
            (gensec, gensec_attribute),
            (spine, spine_attribute),
            (first_point, first_attribute),
            (second_point, second_attribute),
            (datum, datum_attribute),
            (wall, wall_attribute),
        ]);
        source.children.insert(world_root, vec![site]);
        source.children.insert(site, vec![gensec, wall]);
        source.children.insert(gensec, vec![spine, datum]);
        source
            .children
            .insert(spine, vec![first_point, second_point]);
        let source: Arc<dyn TransformFactSource> = Arc::new(source);

        let spine_local = get_local_mat4_with_source(first_point, source.clone())
            .await
            .expect("SPINE local")
            .expect("SPINE matrix");
        assert!(spine_local.is_finite());

        let datum_local = get_local_mat4_with_source(datum, source.clone())
            .await
            .expect("GENSEC local")
            .expect("GENSEC matrix");
        assert!((datum_local.w_axis.z - 5.0).abs() < 1.0e-9);

        let wall_local = get_local_mat4_with_source(wall, source.clone())
            .await
            .expect("WALL local")
            .expect("WALL matrix");
        assert!(wall_local.is_finite());

        let world = get_world_mat4_with_source(wall, source)
            .await
            .expect("world transform")
            .expect("world matrix");
        assert!((world.w_axis.x - 10.0).abs() < 1.0e-9);
        assert!((world.w_axis.y - 2.0).abs() < 1.0e-9);
    }
}
