fn::hvac_shape($pe) == 'CIRC';
fn::pre_is_type($pe, 'DAMP');
fn::next_is_type($pe, 'DAMP');
fn::pre_is_type($pe, 'MESH');
fn::next_is_type($pe, 'MESH');
fn::prev_stype_is_fjfd($pe);
fn::next_stype_is_fjfd($pe);