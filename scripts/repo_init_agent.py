import os

GITHUB_CI_TEMPLATE = """name: Codex Booster CI/CD

on:
  push:
    branches: [ 'main' ]

jobs:
  build-and-deploy:
    runs-on: ubuntu-latest
    steps:
      - name: Checkout code
        uses: actions/checkout@v3
      - name: Set up Python
        uses: actions/setup-python@v4
        with:
          python-version: 3.10
      - name: Install backend
        run: |
          pip install -r backend/requirements.txt
      - name: Set up Node.js
        uses: actions/setup-node@v3
        with:
          node-version: 18
      - name: Install frontend
        run: |
          cd frontend
          npm ci
      - name: Run tests
        run: |
          pytest
          cd frontend && npm test
      - name: Deploy
        run: |
          curl -X POST https://your-deploy-endpoint/api/deploy \
               -H 'Authorization: Bearer ${{ secrets.CI_TOKEN }}'
"""

GITLAB_CI_TEMPLATE = """stages:
  - test
  - build
  - deploy

build:
  script:
    - pip install -r backend/requirements.txt
    - cd frontend && npm install

test:
  script:
    - pytest
    - cd frontend && npm test

deploy:
  script:
    - curl -X POST https://your-deploy-endpoint/api/deploy
"""

def scaffold_cicd(repo_path: str, provider: str = 'github') -> None:
    """Scaffold CI/CD configuration for the given provider."""
    if provider == 'github':
        ci_path = os.path.join(repo_path, '.github', 'workflows')
        os.makedirs(ci_path, exist_ok=True)
        with open(os.path.join(ci_path, 'codex_booster.yml'), 'w') as f:
            f.write(GITHUB_CI_TEMPLATE)
    elif provider == 'gitlab':
        with open(os.path.join(repo_path, '.gitlab-ci.yml'), 'w') as f:
            f.write(GITLAB_CI_TEMPLATE)
    else:
        raise ValueError('Unsupported provider: ' + provider)
