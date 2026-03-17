# GitHub Setup

## 1. Create repository on GitHub

1. Go to https://github.com/new
2. Repository name: `aios-project` (or your choice)
3. Description: `AI-Native Operating System - replaces traditional OS with AI-first interface`
4. Select **Public**
5. **Do not** check "Add a README file" etc. (local repo already has content)

## 2. Add remote and push

```bash
cd path/to/aios-project

git remote add origin https://github.com/minsu18/aios-project.git

git branch -M main
git push -u origin main
```

## 3. Using SSH

```bash
git remote add origin git@github.com:minsu18/aios-project.git
git push -u origin main
```

## 4. Using GitHub CLI (gh)

```bash
gh repo create aios-project --public --source=. --remote=origin --push
```
