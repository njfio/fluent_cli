FROM alpine:latest

RUN apk update && apk upgrade

# Install dependencies
RUN apk add py3-gunicorn
RUN apk add py3-gevent
RUN apk add py3-requests
RUN apk add py3-beautifulsoup4
RUN apk add py3-matplotlib
RUN apk add coreutils
RUN apk add jq
RUN apk add pandoc
RUN apk add neo4j

#install Python and Ruby
RUN apk add --no-cache build-base bash git curl wget python3 python3-dev py3-pip \
                   ruby ruby-dev ruby-irb ruby-rdoc ruby-json ruby-bigdecimal
RUN apk add screen

# Install Rust
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

# Install Fluent CLI
RUN cargo install --force --git https://github.com/njfio/fluent_cli --branch v.0.5.5-feature-docker-image \
    fluent
# Install Amber CLI
RUN cargo install --git https://github.com/fpco/amber amber

# Install Flask
RUN apk add py3-flask
RUN mkdir /app
RUN mkdir /app/frontend

RUN cp ~/.cargo/git/checkouts/fluent_cli-*/*/frontend.py /app/app.py
RUN cp ~/.cargo/git/checkouts/fluent_cli-*/*/front_end_index.html /app/frontend/index.html
RUN cp ~/.cargo/git/checkouts/fluent_cli-*/*/front_end_index.html /app/index.html

RUN mkdir /.fluent/
RUN mkdir /.fluent/state_store/
RUN mkdir /.fluent/example_configurations/

RUN cp ~/.cargo/git/checkouts/fluent_cli-*/*/default_config_test.json /.fluent/
RUN cp ~/.cargo/git/checkouts/fluent_cli-*/*/fluent_autocomplete.sh /.fluent/
RUN cp ~/.cargo/git/checkouts/fluent_cli-*/*/amber.yaml /.fluent/

RUN cp ~/.cargo/git/checkouts/fluent_cli-*/*/fluent-env/start-neo4j.sh /.fluent/
RUN chmod +x /.fluent/start-neo4j.sh
RUN cp ~/.cargo/git/checkouts/fluent_cli-*/*/fluent-env/start-flask.sh /.fluent/
RUN chmod +x /.fluent/start-flask.sh
RUN cp ~/.cargo/git/checkouts/fluent_cli-*/*/fluent-env/start-combined.sh /.fluent/
RUN chmod +x /.fluent/start-combined.sh

RUN cp ~/.cargo/git/checkouts/fluent_cli-*/*/example_configurations/*.json /.fluent/example_configurations


EXPOSE 5000
EXPOSE 7474
EXPOSE 7687
EXPOSE 8000

RUN echo "dbms.default_database=neo4j" >> /etc/neo4j/neo4j.conf
RUN echo "dbms.connector.bolt.enabled=true" >> /etc/neo4j/neo4j.conf
RUN echo "dbms.connector.bolt.listen_address=:7687" >> /etc/neo4j/neo4j.conf
RUN echo "dbms.default_listen_address=0.0.0.0" >> /etc/neo4j/neo4j.conf
#RUN echo "dbms.connector.bolt.tls_level=OPTIONAL" >> /etc/neo4j/neo4j.conf

RUN neo4j-admin set-default-admin neo4j
RUN neo4j-admin set-initial-password system2024!

#CMD ["/.fluent/start-neo4j.sh"]
#CMD ["/.fluent/start-flask.sh"]
# /.fluent/start-combined.sh

WORKDIR /app
CMD ["/.fluent/start-combined.sh"]
#ENTRYPOINT ["bash"]
RUN echo "source /.fluent/fluent_autocomplete.sh" >> ~/.bashrc