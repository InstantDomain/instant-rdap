#/bin/sh

tool_params=
tool_dir=$(pwd)/rdap-conformance-tool/tool/bin
test_output=$(pwd)/test_output

run_test() {
	(cd $tool_dir

	java -jar rdapct-1.0.jar -c ./rdapct-config.json $tool_params http://localhost:11000/$1
	retval=$?

	mv results/*.json $test_output/$(echo $1 | sed 's;/;_;g').json
	exit $retval
	)

	success=$?
	echo "$success: test of $1"

	export tool_params="--use-local-datasets"
}

git submodule init && git submodule update
mkdir -p test_output
rm -rf $tool_dir/results

cargo run & pid=$!
trap "kill $pid;" INT HUP

for i in rdap/domain/* rdap/nameserver/* rdap/entity/*; do 
	run_test $i
done

# The conformance tool doesn't support IP or search queries.
# Leaving these here for reference.
#
#run_test rdap/ip/193.0.0.0
#run_test rdap/ip/193.0.0.0/24
#run_test rdap/ip/3.3.3.3
#run_test rdap/ip/2001:db8::
#run_test rdap/ip/2001:db8::0

kill $pid
