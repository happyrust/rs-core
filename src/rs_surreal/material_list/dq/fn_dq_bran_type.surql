DEFINE FUNCTION fn::dq_bran_type($pe:record) {
    let $first_spre = (select value in from only (select * from $pe<-pe_owner[where in.noun != 'ATTA'] order by order_num) limit 1).refno.SPRE.refno.NAME;
    return
    if $first_spre != NONE && string::contains(string::uppercase($first_spre),'LADDER') { '梯架' }
    else if $first_spre!= NONE && string::contains(string::uppercase($first_spre),'TROUGH') { '实底托盘' }
    else if $first_spre!= NONE && string::contains(string::uppercase($first_spre),'VENTILATED') { '带孔托盘' }
    else if $first_spre!= NONE && string::contains(string::uppercase($first_spre),'RISER') { '竖梯' }
    else if $first_spre!= NONE && string::contains(string::uppercase($first_spre),'DIVIDER') { '分隔板' } else { '' }
};