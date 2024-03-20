
//加强筋型材
insert into washer_len [

    {
        "type": "",
        "shape" : "RECT",
        "exclude_prev_types": ["DAMP", "MESH"],
        "prev_types": [],
        "exclude_next_types": [],
        "next_types": ["FJFD"],
        "exclude_mat": mat:6,
        "expression": "cal_1", //(Fl_Len + FL_Width*8/1000)/2
    }
]