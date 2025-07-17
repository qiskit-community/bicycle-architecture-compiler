# Copyright contributors to the Bicycle Architecture Compiler project
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#     http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

{
    i+=$5;
    t+=$6;
    a+=$7;
    m+=$8;
    j+=$9
}
END{
    print "idles,automorphisms,measurements,joint measurements,t_injs";
    printf "%i & %i & %i & %i & %i\n", i,a,m,j,t;
}
