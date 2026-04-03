FROM quay.io/centos/centos:stream9

# A version field
ENV DOCKERFILE_VERSION=20240926.v05

# dnf config-manager isn't available at first, and
# we need it to install the CRB repo below.
RUN dnf -y install 'dnf-command(config-manager)'

# What used to be powertools is now called "CRB".
# We need it for some of the packages installed below.
# https://docs.fedoraproject.org/en-US/epel/
RUN dnf config-manager --set-enabled crb
RUN dnf -y install \
    https://dl.fedoraproject.org/pub/epel/epel-release-latest-9.noarch.rpm \
    https://dl.fedoraproject.org/pub/epel/epel-next-release-latest-9.noarch.rpm

RUN dnf -y --nobest install \
    bison \
    ccache \
    cmake \
    diffutils \
    flex \
    gcc \
    gcc-c++ \
    git \
    jq \
    libpcap-devel \
    make \
    openssl \
    openssl-devel \
    procps-ng \
    tar \
    which \
    zlib-devel \
  && dnf clean all && rm -rf /var/cache/dnf

# Set the crypto policy to allow SHA-1 certificates - which we have in our tests
RUN dnf -y --nobest install crypto-policies-scripts && update-crypto-policies --set LEGACY

WORKDIR	/apps
RUN git clone http://mod.lge.com/hub/timpani/sample-apps.git

WORKDIR	/apps/sample-apps/build
RUN cmake ..
RUN cmake --build .

ENTRYPOINT [ "/apps/sample-apps/build/sample_apps" ]
