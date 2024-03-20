insert into fl_table [
  {
    "id": 1,
    "code": "FJ25",
    "type": "L25X3",
    "weight_scale": 1.124
  },
  {
    "id": 2,
    "code": "FJ303",
    "type": "L30X3",
    "weight_scale": 1.373
  },
  {
    "id": 3,
    "code": "FJ30",
    "type": "L30X4",
    "weight_scale": 1.786
  },
  {
    "id": 4,
    "code": "FJ35",
    "type": "L35X4",
    "weight_scale": 1.656
  },
  {
    "id": 5,
    "code": "FJ40",
    "type": "L40X4",
    "weight_scale": 2.422
  },
  {
    "id": 6,
    "code": "FJ45",
    "type": "L45X4",
    "weight_scale": 2.736
  },
  {
    "id": 7,
    "code": "FJ50",
    "type": "L50X5",
    "weight_scale": 3.77
  },
  {
    "id": 8,
    "code": "FJ60",
    "type": "L60X6",
    "weight_scale": 5.369
  },
  {
    "id": 9,
    "code": "FJ608",
    "type": "L60X8",
    "weight_scale": 7.081
  },
  {
    "id": 10,
    "code": "FJ6550",
    "type": "L65X50X6",
    "weight_scale": 5.134
  },
  {
    "id": 11,
    "code": "FJ65",
    "type": "L65X6",
    "weight_scale": 5.3694
  },
  {
    "id": 12,
    "code": "FJ63",
    "type": "L63X6",
    "weight_scale": 5.721
  },
  {
    "id": 13,
    "code": "FJ70",
    "type": "L70X7",
    "weight_scale": 7.398
  },
  {
    "id": 14,
    "code": "FJ7550",
    "type": "L75X50X6",
    "weight_scale": 5.699
  },
  {
    "id": 15,
    "code": "FJ75",
    "type": "L75X7",
    "weight_scale": 7.976
  },
  {
    "id": 16,
    "code": "FJ8060",
    "type": "L80X60X6",
    "weight_scale": 6.311
  },
  {
    "id": 17,
    "code": "FJ80",
    "type": "L80X8",
    "weight_scale": 9.658
  },
  {
    "id": 18,
    "code": "FJ8010",
    "type": "L80X10",
    "weight_scale": 11.874
  },
  {
    "id": 19,
    "code": "FJ90",
    "type": "L90X9",
    "weight_scale": 5.661
  },
  {
    "id": 20,
    "code": "FJ9056",
    "type": "L90X56X5",
    "weight_scale": 5.661
  },
  {
    "id": 21,
    "code": "FJ10065",
    "type": "L100X65X8",
    "weight_scale": 9.8596
  },
  {
    "id": 22,
    "code": "FJ10080",
    "type": "L100X80X8",
    "weight_scale": 10.946
  },
  {
    "id": 23,
    "code": "FJ100",
    "type": "L100X8",
    "weight_scale": 12.276
  },
  {
    "id": 24,
    "code": "FB253",
    "type": "FB25X3",
    "weight_scale": 0.59
  },
  {
    "id": 25,
    "code": "FB254",
    "type": "FB25X4",
    "weight_scale": 0.78
  },
  {
    "id": 26,
    "code": "FB304",
    "type": "FB30X4",
    "weight_scale": 0.94
  },
  {
    "id": 27,
    "code": "FB305",
    "type": "FB30X5",
    "weight_scale": 1.18
  },
  {
    "id": 28,
    "code": "FB354",
    "type": "FB35X4",
    "weight_scale": 1.1
  },
  {
    "id": 29,
    "code": "FB404",
    "type": "FB40X4",
    "weight_scale": 1.26
  },
  {
    "id": 30,
    "code": "FB405",
    "type": "FB40X5",
    "weight_scale": 1.57
  },
  {
    "id": 31,
    "code": "FB505",
    "type": "FB50X5",
    "weight_scale": 1.96
  },
  {
    "id": 32,
    "code": "FB606",
    "type": "FB60X6",
    "weight_scale": 2.826
  },
  {
    "id": 33,
    "code": "FB608",
    "type": "FB60X8",
    "weight_scale": 3.77
  },
  {
    "id": 34,
    "code": "FB6010",
    "type": "FB60X10",
    "weight_scale": 4.71
  },
  {
    "id": 35,
    "code": "FB808",
    "type": "FB80X8",
    "weight_scale": 5.02
  },
  {
    "id": 36,
    "code": "FB8010",
    "type": "FB80X10",
    "weight_scale": 6.28
  },
  {
    "id": 37,
    "code": "FB8012",
    "type": "FB80X12",
    "weight_scale": 7.54
  }
];

DEFINE FUNCTION fn::hvac_fl_type($pe: record) {
    return fn::into_str($pe.refno.DESP[57]);
};

DEFINE FUNCTION fn::hvac_fl_spec($pe: record) {
    return $pe.refno.DESP[67];
};

DEFINE FUNCTION fn::hvac_fl_type_spec($pe: record) {
    return string::concat(fn::hvac_fl_type($pe), string::slice(<string> fn::hvac_fl_spec($pe), 0, -1));
};

DEFINE FUNCTION fn::hvac_shape_fl_type($pe: record) {
    return string::concat(fn::hvac_shape($pe), fn::hvac_fl_type($pe));
};

DEFINE FUNCTION fn::hvac_fl_name($pe: record) {
    let $f = fn::hvac_fl_type_spec($pe);
    let $n = (select value type from fl_table where code = $f)[0];
    return if $n == none {
        "unset"
    } else {
        $n
    };
};


DEFINE FUNCTION fn::hvac_flw($pe: record) {
    return $pe.refno.DESP[20];
};

DEFINE FUNCTION fn::hvac_next_flw($pe: record) {
    return fn::next_pe($pe).refno.DESP[20];
};

DEFINE FUNCTION fn::hvac_prev_flw($pe: record) {
    return fn::prev_pe($pe).refno.DESP[20];
};


DEFINE FUNCTION fn::hvac_fl_len($pe: record) {
    let $f = fn::hvac_shape_fl_type($pe);
    let $m = fn::hvac_mat_id($pe);
    let $w = fn::hvac_width($pe);
    let $h = fn::hvac_height($pe);

    return if $m != 6 {
        let $flw = fn::hvac_flw($pe);
        return if $f == "RECTFJ" {
            (($w + $h + $flw * 2 + 3) * 4 ) / 1000
        } else if $f == "RECTFB" {
            (($w + $h + $flw * 2 + 5) * 4 ) / 1000
        } else if $f == "CIRCFJ" {
            (($w + $flw * 2 + 3) * 6.28 ) / 1000
        } else if $f == "CIRCFB" {
            (($w + $flw * 2 + 5) * 6.28 ) / 1000
        } else {
            0
        };
    } else {
         let $prev_is_damp = fn::pre_is_type($pe, "DAMP");
         let $next_is_damp = fn::next_is_type($pe, "DAMP");

         return if $next_is_damp  {
            //string(([Width] + [Height] + [Next_FLW] * 2 + 3) * 2 / 1000 , 'D2')
             (($w + $h + fn::hvac_next_flw($pe) * 2 + 3) * 2 ) / 1000
         } else if $prev_is_damp {
            (($w + $h + fn::hvac_prev_flw($pe) * 2 + 3) * 2 ) / 1000
         } else {
            0
         };
    };
};


DEFINE FUNCTION fn::hvac_fl_wei($pe: record) {
    let $f = fn::hvac_fl_type_spec($pe);
    let $scale = (select value weight_scale from fl_table where code = $f)[0];
    return fn::hvac_fl_len($pe) * ( if $scale == none { 0.0 } else { $scale } );
};



