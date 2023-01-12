#
# Licensed to the Apache Software Foundation (ASF) under one
# or more contributor license agreements.  See the NOTICE file
# distributed with this work for additional information
# regarding copyright ownership.  The ASF licenses this file
# to you under the Apache License, Version 2.0 (the
# "License"); you may not use this file except in compliance
# with the License.  You may obtain a copy of the License at
#
#   http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing,
# software distributed under the License is distributed on an
# "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
# KIND, either express or implied.  See the License for the
# specific language governing permissions and limitations
# under the License.
#

FROM ttbb/compile:rust AS build
COPY . /opt/compile
WORKDIR /opt/compile
RUN /root/.cargo/bin/cargo build --release

FROM shoothzj/base

WORKDIR /opt/minio-dashboard

COPY --from=build /opt/compile/target/release/minio-cli /opt/minio-dashboard/minio-cli
COPY --from=build /opt/compile/target/release/minio-dashboard /opt/minio-dashboard/minio-dashboard

RUN wget -q https://github.com/paas-dashboard/minio-dashboard-portal/releases/download/latest/minio-dashboard-portal.tar.gz && \
    tar -xzf minio-dashboard-portal.tar.gz && \
    rm -rf minio-dashboard-portal.tar.gz

EXPOSE 10005

CMD ["/usr/bin/dumb-init", "/opt/minio-dashboard/minio-dashboard"]
