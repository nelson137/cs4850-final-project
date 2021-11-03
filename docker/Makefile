TAG        = chat-boat

BASE_IMG   = base.Dockerfile
SERVER_IMG = server.Dockerfile
CLIENT_IMG = client.Dockerfile

ALL_IMGS   = $(BASE_IMG) $(SERVER_IMG) $(CLIENT_IMG)

define docker_build
$(shell echo docker build -t '$(TAG)/$(1)' -f '$(1).Dockerfile' ..)
endef

DOCKER_RUN = docker run --rm --network=host -it

.PHONY: all $(ALL_IMGS) server client

all : $(SERVER_IMG) $(CLIENT_IMG)

server : $(SERVER_IMG)
	$(DOCKER_RUN) '$(TAG)/$(@)' --name server

client : $(CLIENT_IMG)
	$(DOCKER_RUN) '$(TAG)/$(@)' --name client

$(BASE_IMG) :
	$(call docker_build,$(@:%.Dockerfile=%))

$(SERVER_IMG) : $(BASE_IMG)
	$(call docker_build,$(@:%.Dockerfile=%))

$(CLIENT_IMG) : $(BASE_IMG)
	$(call docker_build,$(@:%.Dockerfile=%))
