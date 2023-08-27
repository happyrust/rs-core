use crate::prim_geo::revolution::Revolution;
use crate::prim_geo::sbox::SBox;
use crate::shape::pdms_shape::BrepShapeTrait;

#[test]
fn test_key_pts() {

    let cube = SBox::default();
    let key_pts = cube.key_points();
    dbg!(key_pts);

    let revol = Revolution::default();
    dbg!(revol.key_points());



}