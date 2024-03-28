let $a = array::flatten(select value array::flatten([id, <-pe_owner.in]) from $refnos);
select [cata_hash, type::thing('inst_info', cata_hash).id!=none] as k, array::group(id) as v from $a group by k;