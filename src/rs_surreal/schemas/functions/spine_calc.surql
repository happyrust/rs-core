// SPINE路径长度计算函数
// 支持POINSP直线段和CURVE弧线段的混合路径

remove function fn::calc_spine_length;
DEFINE FUNCTION fn::calc_spine_length($spine_id: record) {
    // 获取SPINE下的所有子元素(POINSP和CURVE)，按order_num排序
    let $children = (
        SELECT in.noun as type,
               in.refno.POS as pos,
               in.refno.RADI as radius,
               in.refno.CURTYP as curve_type,
               order_num
        FROM $spine_id<-pe_owner
        ORDER BY order_num
    );

    // 如果没有子元素，返回0
    IF array::len($children) == 0 {
        RETURN 0.0;
    };

    let $total_length = 0.0;
    let $i = 0;

    // 遍历所有子元素，计算每段的长度
    WHILE $i < array::len($children) - 1 {
        let $current = $children[$i];
        let $next = $children[$i + 1];

        // POINSP to POINSP: 直线段
        IF $current.type == 'POINSP' AND $next.type == 'POINSP' {
            let $dist = vector::distance::euclidean($current.pos, $next.pos);
            LET $total_length = $total_length + $dist;
            LET $i = $i + 1;
        }
        // POINSP to CURVE to POINSP: 弧线段
        ELSE IF $current.type == 'POINSP' AND $next.type == 'CURVE' {
            // 获取弧线后的下一个点
            IF $i + 2 < array::len($children) {
                let $after_curve = $children[$i + 2];
                IF $after_curve.type == 'POINSP' {
                    // 计算弧长
                    let $p1 = $current.pos;
                    let $p2 = $next.pos;  // 弧线上的点
                    let $p3 = $after_curve.pos;

                    // 对于THRU类型的弧线，需要计算实际弧长
                    IF $next.curve_type == 'THRU' {
                        // 使用三点计算圆心和半径
                        let $arc_length = fn::calc_arc_length_thru($p1, $p2, $p3, $next.radius);
                        LET $total_length = $total_length + $arc_length;
                    } ELSE IF $next.radius {
                        // 如果有半径，使用半径计算
                        // 计算圆心角
                        let $v1 = [$p1[0] - $p2[0], $p1[1] - $p2[1], $p1[2] - $p2[2]];
                        let $v2 = [$p3[0] - $p2[0], $p3[1] - $p2[1], $p3[2] - $p2[2]];
                        let $angle = fn::vector_angle($v1, $v2);
                        let $arc_length = $next.radius * $angle;
                        LET $total_length = $total_length + $arc_length;
                    } ELSE {
                        // 如果没有半径信息，使用两段直线距离之和作为近似
                        let $d1 = vector::distance::euclidean($p1, $p2);
                        let $d2 = vector::distance::euclidean($p2, $p3);
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

// 辅助函数：计算两个向量之间的夹角（弧度）
remove function fn::vector_angle;
DEFINE FUNCTION fn::vector_angle($v1: array, $v2: array) {
    let $dot = $v1[0] * $v2[0] + $v1[1] * $v2[1] + $v1[2] * $v2[2];
    let $mag1 = math::sqrt($v1[0] * $v1[0] + $v1[1] * $v1[1] + $v1[2] * $v1[2]);
    let $mag2 = math::sqrt($v2[0] * $v2[0] + $v2[1] * $v2[1] + $v2[2] * $v2[2]);

    IF $mag1 == 0 OR $mag2 == 0 {
        RETURN 0.0;
    };

    let $cos_angle = $dot / ($mag1 * $mag2);
    // 限制在[-1, 1]范围内，避免数值误差
    IF $cos_angle > 1 {
        LET $cos_angle = 1;
    } ELSE IF $cos_angle < -1 {
        LET $cos_angle = -1;
    };

    RETURN math::acos($cos_angle);
};

// 辅助函数：通过三点计算THRU类型弧线的长度
remove function fn::calc_arc_length_thru;
DEFINE FUNCTION fn::calc_arc_length_thru($p1: array, $p2: array, $p3: array, $given_radius: float) {
    // 如果给定了半径
    IF $given_radius AND $given_radius > 0 {
        // 计算弦长
        let $chord = vector::distance::euclidean($p1, $p3);
        // 计算圆心角 (使用弦长和半径)
        IF $chord <= 2 * $given_radius {
            let $angle = 2 * math::asin($chord / (2 * $given_radius));
            RETURN $given_radius * $angle;
        };
    };

    // 如果没有半径或计算失败，使用两段距离之和
    let $d1 = vector::distance::euclidean($p1, $p2);
    let $d2 = vector::distance::euclidean($p2, $p3);
    RETURN $d1 + $d2;
};

// 获取GENSEC的SPINE总长度
remove function fn::get_gensec_spine_length;
DEFINE FUNCTION fn::get_gensec_spine_length($gensec_id: record) {
    // 查找GENSEC下的SPINE
    let $spine = (
        SELECT in.id as spine_id
        FROM $gensec_id<-pe_owner
        WHERE in.noun = 'SPINE'
        LIMIT 1
    );

    IF array::len($spine) == 0 {
        RETURN 0.0;
    };

    // 计算SPINE路径长度
    RETURN fn::calc_spine_length($spine[0].spine_id);
};