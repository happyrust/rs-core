LET $pes = (SELECT id, owner, noun, refno FROM pe where !type::is::record(owner));

for $p in $pes{
    //update owner record link
    (update (<record> $p['id']) set owner=(type::thing("pe", $p['owner'])));
};


for $p in $pes{
    //update refno record link
    (update (<record> $p['id']) set refno=(type::thing($p["noun"], $p['refno'])));
};