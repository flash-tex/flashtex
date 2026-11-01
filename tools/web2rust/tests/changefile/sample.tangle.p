{1:}program sample;label{2:}10;{:2}const{3:}bufsize=100;linemax=15;
extra=7;{:3}var{4:}k:integer;total:integer;
{:4}{7:}procedure add(n:integer);begin total:=total+(n+n);end;
{:7}{9:}procedure twice;begin add(total);end;{:9}begin{5:}k:=0;total:=0;
k:=256;k:=257;total:=258;{:5}{6:}total:=259;{:6};
{8:}for k:=1 to 10 do add(k);
if total>linemax then total:=linemax else total:=total+extra;
total:=total+k+k;{:8};10:end.{:1}
