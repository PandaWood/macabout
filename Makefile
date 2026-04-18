.PHONY: dev run mock build clean

PYTHON := python3
VENV := .venv
VENV_PYTHON := $(VENV)/bin/python

dev:
	@$(PYTHON) -m venv $(VENV) || { \
		echo ""; \
		echo "ERROR: venv creation failed. Install the required packages:"; \
		echo "  sudo apt install python3-venv python3-pip"; \
		echo "Then re-run: make dev"; \
		echo ""; \
		exit 1; \
	}
	$(VENV_PYTHON) -m pip install --upgrade pip -q
	$(VENV_PYTHON) -m pip install -r requirements.txt
	@echo ""
	@echo "Done. Activate with: source .venv/bin/activate"
	@echo "Then run:            python -m macabout --mock"

run:
	$(VENV_PYTHON) -m macabout

mock:
	$(VENV_PYTHON) -m macabout --mock

build:
	./build.sh

clean:
	rm -rf build/ $(VENV) $$(find . -name __pycache__)
