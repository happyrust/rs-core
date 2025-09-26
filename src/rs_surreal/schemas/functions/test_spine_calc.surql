// 测试SPINE计算函数
// 使用模拟数据验证计算逻辑

-- 测试1: 简单的两点直线
let $test_line = {
    children: [
        { type: 'POINSP', pos: [0, 0, 0] },
        { type: 'POINSP', pos: [100, 0, 0] }
    ]
};

-- 期望结果: 100
let $result1 = fn::calc_spine_length_test($test_line.children);
assert $result1 == 100.0;

-- 测试2: 三点两段直线
let $test_multi_line = {
    children: [
        { type: 'POINSP', pos: [0, 0, 0] },
        { type: 'POINSP', pos: [100, 0, 0] },
        { type: 'POINSP', pos: [100, 100, 0] }
    ]
};

-- 期望结果: 100 + 100 = 200
let $result2 = fn::calc_spine_length_test($test_multi_line.children);
assert $result2 == 200.0;

-- 测试3: 包含弧线的路径（模拟gensec.txt的数据）
let $test_arc = {
    children: [
        { type: 'POINSP', pos: [12635, -25862, 1950] },
        { type: 'POINSP', pos: [12785, -25862, 1950] },
        { type: 'CURVE', pos: [12884, -25903.01, 1950], radius: 140, curve_type: 'THRU' },
        { type: 'POINSP', pos: [12925, -26002, 1950] },
        { type: 'POINSP', pos: [12925, -26152, 1950] },
        { type: 'POINSP', pos: [12925, -29152, 1950] },
        { type: 'POINSP', pos: [12925, -30652, 1950] }
    ]
};

-- 计算各段长度
-- 段1: (12635,-25862,1950) -> (12785,-25862,1950) = 150mm
-- 段2: 弧线 (12785,-25862,1950) -> (12925,-26002,1950), 半径140mm
-- 段3: (12925,-26002,1950) -> (12925,-26152,1950) = 150mm
-- 段4: (12925,-26152,1950) -> (12925,-29152,1950) = 3000mm
-- 段5: (12925,-29152,1950) -> (12925,-30652,1950) = 1500mm

let $result3 = fn::calc_spine_length_test($test_arc.children);
return {
    test1_simple_line: $result1,
    test2_multi_line: $result2,
    test3_arc_path: $result3,
    expected_ranges: {
        test3_min: 4800,
        test3_max: 5000
    }
};

-- 辅助测试函数（不依赖数据库记录）
remove function fn::calc_spine_length_test;
DEFINE FUNCTION fn::calc_spine_length_test($children: array) {
    IF array::len($children) == 0 {
        RETURN 0.0;
    };

    let $total_length = 0.0;
    let $i = 0;

    WHILE $i < array::len($children) - 1 {
        let $current = $children[$i];
        let $next = $children[$i + 1];

        -- POINSP to POINSP: 直线段
        IF $current.type == 'POINSP' AND $next.type == 'POINSP' {
            let $dist = vector::distance::euclidean($current.pos, $next.pos);
            LET $total_length = $total_length + $dist;
            LET $i = $i + 1;
        }
        -- POINSP to CURVE to POINSP: 弧线段
        ELSE IF $current.type == 'POINSP' AND $next.type == 'CURVE' {
            IF $i + 2 < array::len($children) {
                let $after_curve = $children[$i + 2];
                IF $after_curve.type == 'POINSP' {
                    -- 计算弧长
                    let $p1 = $current.pos;
                    let $p3 = $after_curve.pos;

                    IF $next.radius AND $next.radius > 0 {
                        -- 计算弦长
                        let $chord = vector::distance::euclidean($p1, $p3);
                        -- 计算圆心角
                        IF $chord <= 2 * $next.radius {
                            let $angle = 2 * math::asin($chord / (2 * $next.radius));
                            let $arc_length = $next.radius * $angle;
                            LET $total_length = $total_length + $arc_length;
                        } ELSE {
                            -- 弦长大于直径，使用近似
                            let $d1 = vector::distance::euclidean($p1, $next.pos);
                            let $d2 = vector::distance::euclidean($next.pos, $p3);
                            LET $total_length = $total_length + $d1 + $d2;
                        };
                    } ELSE {
                        -- 没有半径，使用两段距离之和
                        let $d1 = vector::distance::euclidean($p1, $next.pos);
                        let $d2 = vector::distance::euclidean($next.pos, $p3);
                        LET $total_length = $total_length + $d1 + $d2;
                    };
                    LET $i = $i + 2;
                } ELSE {
                    LET $i = $i + 1;
                };
            } ELSE {
                LET $i = $i + 1;
            };
        } ELSE {
            LET $i = $i + 1;
        };
    };

    RETURN math::fixed($total_length, 3);
};