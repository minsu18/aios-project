# GitHub 공개 프로젝트 설정

## 1. GitHub에서 저장소 생성

1. https://github.com/new 접속
2. Repository name: `aios-project` (또는 원하는 이름)
3. Description: `AI-Native Operating System - replaces traditional OS with AI-first interface`
4. **Public** 선택
5. **"Add a README file" 등 체크하지 않고** Create (로컬에 이미 있음)

## 2. 원격 저장소 연결 및 푸시

```bash
cd /Users/cheonminsu/Projects/AgRevo_with_Claude/aios-project

# YOUR_USERNAME을 GitHub 사용자명으로 교체
git remote add origin https://github.com/YOUR_USERNAME/aios-project.git

git branch -M main
git push -u origin main
```

## 3. SSH 사용 시

```bash
git remote add origin git@github.com:YOUR_USERNAME/aios-project.git
git push -u origin main
```

## 4. GitHub CLI (gh) 사용 시

```bash
gh repo create aios-project --public --source=. --remote=origin --push
```
