// 设备清单
// let $refno = [pe:24381_57828,pe:24383_76416];

select
id as id,
string::slice(refno.NAME,1) as name, // 设备位号
'' as room_code, // 房间号 todo
fn::default_names(array::flatten([<-pe_owner[where in.noun='NOZZ']<-pe,  <-pe_owner.in<-pe_owner[where in.noun='NOZZ'].in])) as nozz_name, // 管口号
array::clump(array::flatten([<-pe_owner[where in.noun='NOZZ']<-pe.refno.POS,  <-pe_owner.in<-pe_owner[where in.noun='NOZZ'].in.refno.POS]),3) as nozz_pos, // 管口坐标

(select value if (name == NONE) { '' } else { string::slice(name, 1) } from array::flatten([<-pe_owner[where in.noun='NOZZ']<-pe,  <-pe_owner.in<-pe_owner[where in.noun='NOZZ'].in])) as nozz_cref // 相连管道编号
from $refno; // 24381_57828  24383_76416