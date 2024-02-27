select
id as id,
string::slice(NAME,1) as name, // 设备位号
'' as room_code, // 房间号 todo
fn::default_names(array::flatten([REFNO<-pe_owner[where in.noun='NOZZ']<-pe,  REFNO<-pe_owner.in<-pe_owner[where in.noun='NOZZ'].in])) as nozz_name, // 管口号
array::clump(array::flatten([REFNO<-pe_owner[where in.noun='NOZZ']<-pe.refno.POS,  REFNO<-pe_owner.in<-pe_owner[where in.noun='NOZZ'].in.refno.POS]),3) as nozz_pos, // 管口坐标

(select value if (name == NONE) { '' } else { string::slice(name, 1) } from array::flatten([REFNO<-pe_owner[where in.noun='NOZZ']<-pe,  REFNO<-pe_owner.in<-pe_owner[where in.noun='NOZZ'].in])) as nozz_cref // 相连管道编号
from EQUI limit 5; // 24381_57828  24383_76416