FROM alpine:latest

RUN apk update && apk upgrade

RUN apk add jq
RUN apk add pandoc

# Install dependencies
RUN apk add --no-cache build-base bash git curl wget python3 python3-dev py3-pip \
                   ruby ruby-dev ruby-irb ruby-rdoc ruby-json ruby-bigdecimal

# Install Rust
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

# Install Fluent CLI
RUN cargo install --git https://github.com/njfio/fluent_cli --branch v.0.5.5-feature-docker-image fluent
# Install Amber CLI
RUN cargo install --git https://github.com/fpco/amber amber
# Install Flask
RUN apk add py3-flask
RUN mkdir /app
RUN mkdir /app/frontend

RUN cp ~/.cargo/git/checkouts/fluent_cli-*/*/frontend.py /app/frontend/app.py
RUN cp ~/.cargo/git/checkouts/fluent_cli-*/*/front_end_index.html /app/frontend/index.html

EXPOSE 5000

CMD ["flask", "run", "--host=0.0.0.0"]

RUN mkdir /.fluent/
RUN cp ~/.cargo/git/checkouts/fluent_cli-*/*/default_config_test.json /.fluent/
RUN cp ~/.cargo/git/checkouts/fluent_cli-*/*/fluent_autocomplete.sh /.fluent/
RUN cp ~/.cargo/git/checkouts/fluent_cli-*/*/amber.yaml /.fluent/

# Set up Entrypoint
ENTRYPOINT ["bash"]
RUN echo "source /.fluent/fluent_autocomplete.sh" >> ~/.bashrc