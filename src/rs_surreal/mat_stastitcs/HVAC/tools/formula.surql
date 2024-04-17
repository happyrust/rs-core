
//计算数量的公式1
DEFINE FUNCTION fn::cal_number_formula_1($pe: record, $n: option<int>)  {

    return math::round( (fn::hvac_width($pe, $n) + (fn::hvac_fl_width($pe, $n) * 2.0) + 75.0) / 150.0 )
                                + math::round( ( fn::hvac_height($pe, $n) + (fn::hvac_fl_width($pe, $n) * 2.0) + 75.0 ) / 150.0 ) - 2;
};


DEFINE FUNCTION fn::cal_number_formula_2($d: float)  {
    return (( $d + 75.0 ) / 150.0);
};