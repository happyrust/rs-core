use std::any::TypeId;

use bevy_reflect::{Reflect, Struct, reflect_trait, TypeRegistry, DynamicStruct, Typed, ReflectFromReflect, std_traits::ReflectDefault};
use serde_with::serde_as;
use crate::types::*;
use serde::{Serialize, Deserialize};
use sea_orm::entity::prelude::*;
use serde_with::DisplayFromStr;

#[serde_as]
#[derive(Serialize, Deserialize, Clone, Debug, Default, DeriveEntityModel)]
#[sea_orm(table_name = "PdmsElement")]
#[derive(Reflect)]
#[reflect(DoThing, Default)]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    #[serde_as(as = "DisplayFromStr")]
    pub refno: RefU64,
    // #[serde(serialize_with = "ser_refno_as_ref_type")]
    // #[serde(skip_serializing_if = "is_zero")]
    #[serde_as(as = "DisplayFromStr")]
    pub owner: RefU64,
    pub name: String,
    pub noun: String,
    // #[serde(default)]
    // pub order: u32,
    pub dbnum: i32,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_tag: Option<String>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cata_hash: Option<String>,
}

impl DoThing for Model {
    fn do_thing(&self) -> String {
        format!("{} World!", &self.name)
    }
}


#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}


#[test]
fn test_ele_reflect() {
    let mut data = Model::default();
    data.name = "PdmsElement".to_owned();
    for (i, v) in data.iter_fields().enumerate() {
        let field_name = data.name_at(i).unwrap();
        if let Some(value) = v.downcast_ref::<i32>() {
            println!("{} is a u32 with the value: {}", field_name, *value);
        }
    }

    let mut dynamic_struct = DynamicStruct::default();
    let type_info = <Model as Typed>::type_info();
    dynamic_struct.set_represented_type(Some(type_info));
    dynamic_struct.insert("name", "Test".to_string());

    // First, lets box our type as a Box<dyn Reflect>
    let reflect_value: Box<dyn Reflect> = Box::new(data);
    dbg!(reflect_value.type_name());
    dbg!(reflect_value.type_id());
    

    
    // 
    let reflect_value_test: Box<dyn Reflect> = Box::new(dynamic_struct);
    // dbg!(reflect_value_test.type_name());
    // dbg!(reflect_value_test.type_id());
    
    let mut type_registry: TypeRegistry = TypeRegistry::default();
    type_registry.register::<Model>();
    
    dbg!(TypeId::of::<Model>());


     // Get type data
     let type_id = TypeId::of::<Model>();
     let rfr = type_registry
         .get_type_data::<ReflectFromReflect>(type_id)
         .expect("the FromReflect trait should be registered");

     // Call from_reflect
     let mut dynamic_struct = DynamicStruct::default();
     dynamic_struct.insert("name", "test".to_string());
     let reflected = rfr
         .from_reflect(&dynamic_struct)
         .expect("the type should be properly reflected");


    // The #[reflect] attribute we put on our DoThing trait generated a new `ReflectDoThing` struct, which implements TypeData.
    // This was added to MyType's TypeRegistration.
    let reflect_do_thing = type_registry
        // .get_type_data::<ReflectDoThing>(reflect_value.type_id())
        .get_type_data::<ReflectDoThing>(TypeId::of::<Model>())
        .unwrap();

    // We can use this generated type to convert our `&dyn Reflect` reference to a `&dyn DoThing` reference
    // let my_trait: &dyn DoThing = reflect_do_thing.get(&*reflect_value).unwrap();
    let my_trait: &dyn DoThing = reflect_do_thing.get(&*reflected).unwrap();

    // Which means we can now call do_thing(). Magic!
    println!("{}", my_trait.do_thing());

}

#[reflect_trait]
pub trait DoThing {
    fn do_thing(&self) -> String;
}