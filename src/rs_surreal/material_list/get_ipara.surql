// DEFINE FUNCTION fn::get_ipara($pe:record) {
//     return select value (select value CATR.refno.PARA from only
//     owner.refno.ISPE<-pe_owner<-pe<-pe_owner<-pe[? refno.TYPE = 'SELE'
//     and $parent.owner.refno.TEMP >=refno.ANSW and $parent.owner.refno.TEMP <= refno.MAXA ]<-pe_owner<-pe.refno.*
//     where $parent.owner.refno.HBOR >=ANSW and $parent.owner.refno.HBOR <= MAXA limit 1) from $pe;
// };

// return fn::get_ipara(pe:17496_170719);