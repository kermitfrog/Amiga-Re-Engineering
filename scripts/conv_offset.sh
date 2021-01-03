O=0x06F3D1D8
while true ; do
	read offset
	echo is 
	qalc -b 16 $O+0x$offset | cut -dx -f2
	echo or
	qalc -b 16 $O-0x$offset | cut -dx -f2
	echo '\n'
done
