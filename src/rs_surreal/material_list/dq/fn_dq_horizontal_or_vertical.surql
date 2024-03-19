DEFINE FUNCTION fn::dq_horizontal_or_vertical($pe:record) {
    let $first_spre = (select value in from only (select * from $pe<-pe_owner[where in.noun != 'ATTA'] order by order_num) limit 1).refno.SPRE.refno.NAME;
    return
    if $first_spre != NONE && (string::contains(string::uppercase($first_spre),'RISER') || string::contains(string::uppercase($first_spre),'DIVIDER')) {
        '竖向'
    } else {
        '水平'
    }
};

// return fn::dq_horizontal_or_vertical(pe:24383_84088)

