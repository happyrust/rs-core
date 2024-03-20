DEFINE FUNCTION fn::get_valv_material($pe:record) {
    let $material = ["SS","GS","CS","ACS","ASS","AS","FP"];
    let $id = string::split(($pe->pe_owner.out.refno.NAME)[0],'-')[3];

    return array::at($material,<float>$id -1)
};
