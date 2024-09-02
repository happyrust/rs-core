//定义一些工艺专业的公共函数


// 是否阀门支架
//修改方法名，是否阀门支架的正确翻译

remove function fn::is_valve_support;
DEFINE FUNCTION fn::is_valve_support($pe:record) {
    let $pre = fn::prev_pe($pe);
    
    // 使用 vector::distance::euclidean() 来计算距离
    let $distance = vector::distance::euclidean($pre.refno.POS, $pe.refno.POS);
    
    // 定义一个小的阈值来判断位置是否"相等"
    let $threshold = 0.01;
    
    return if $pre.TYPE == 'ATTA' && $distance < $threshold {
        '是'
    } else {
        '否'
    };
};

// return fn::is_valve_support(pe:24383_66745)