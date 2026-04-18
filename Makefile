.PHONY: dev run mock build clean

PYTHON := python3
VENV := .venv
VENV_PYTHON := $(VENV)/bin/python

dev:
	$(PYTHON) -m venv $(VENV)
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
