# OpenClaw 互助平台实现计划

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 构建一个去中心化的 Agent 任务协作平台，支持任务发布、领取、裁决和积分奖励。

**Architecture:** 前后端分离架构，FastAPI 提供 REST API，React 提供 Web 界面，SQLite/PostgreSQL 存储数据，多 Agent 投票裁决机制。

**Tech Stack:** Python 3.10+, FastAPI, SQLAlchemy, React 18+, SQLite/PostgreSQL, pytest

---

## 文件结构总览

```
mutual-aid-platform/
├── backend/
│   ├── app/
│   │   ├── __init__.py
│   │   ├── main.py              # FastAPI 应用入口
│   │   ├── config.py            # 配置管理
│   │   ├── database.py          # 数据库连接
│   │   ├── models/
│   │   │   ├── __init__.py
│   │   │   ├── user.py          # User 模型
│   │   │   ├── task.py          # Task 模型
│   │   │   ├── submission.py    # Submission 模型
│   │   │   ├── judgment.py      # Judgment 模型
│   │   │   └── transaction.py   # Transaction 模型
│   │   ├── schemas/
│   │   │   ├── __init__.py
│   │   │   ├── user.py          # User Pydantic  schema
│   │   │   ├── task.py          # Task schema
│   │   │   ├── submission.py
│   │   │   ├── judgment.py
│   │   │   └── transaction.py
│   │   ├── api/
│   │   │   ├── __init__.py
│   │   │   ├── users.py         # 用户 API 路由
│   │   │   ├── tasks.py         # 任务 API 路由
│   │   │   ├── judgments.py     # 裁决 API 路由
│   │   │   └── transactions.py  # 交易 API 路由
│   │   ├── services/
│   │   │   ├── __init__.py
│   │   │   ├── user_service.py
│   │   │   ├── task_service.py
│   │   │   ├── judgment_service.py
│   │   │   └── points_service.py
│   │   └── middleware/
│   │       └── auth.py          # 认证中间件
│   ├── tests/
│   │   ├── __init__.py
│   │   ├── conftest.py
│   │   ├── test_users.py
│   │   ├── test_tasks.py
│   │   ├── test_judgments.py
│   │   └── test_points.py
│   ├── requirements.txt
│   └── pytest.ini
├── frontend/
│   ├── src/
│   │   ├── App.tsx
│   │   ├── index.tsx
│   │   ├── components/
│   │   │   ├── TaskList.tsx
│   │   │   ├── TaskDetail.tsx
│   │   │   ├── TaskForm.tsx
│   │   │   ├── UserProfile.tsx
│   │   │   └── JudgmentPanel.tsx
│   │   ├── pages/
│   │   │   ├── Home.tsx
│   │   │   ├── Tasks.tsx
│   │   │   ├── Profile.tsx
│   │   │   └── CreateTask.tsx
│   │   ├── api/
│   │   │   └── client.ts
│   │   └── types/
│   │       └── index.ts
│   ├── package.json
│   └── tsconfig.json
└── docker-compose.yml
```

---

## Chunk 1: 后端基础架构 + 用户模块

### Task 1: 项目初始化

**Files:**
- Create: `backend/requirements.txt`
- Create: `backend/pytest.ini`
- Create: `backend/app/__init__.py`
- Create: `backend/app/config.py`
- Create: `backend/app/database.py`
- Create: `backend/tests/__init__.py`
- Create: `backend/tests/conftest.py`

- [ ] **Step 1: 创建 requirements.txt**

```txt
fastapi==0.109.0
uvicorn[standard]==0.27.0
sqlalchemy==2.0.25
pydantic==2.5.3
pydantic-settings==2.1.0
pytest==7.4.4
pytest-asyncio==0.23.3
httpx==0.26.0
python-jose[cryptography]==3.3.0
passlib[bcrypt]==1.7.4
```

- [ ] **Step 2: 创建 pytest.ini**

```ini
[pytest]
testpaths = tests
asyncio_mode = auto
python_files = test_*.py
python_functions = test_*
```

- [ ] **Step 3: 创建 app/config.py**

```python
from pydantic_settings import BaseSettings
from typing import Optional

class Settings(BaseSettings):
    PROJECT_NAME: str = "OpenClaw Mutual Aid Platform"
    VERSION: str = "0.1.0"
    API_V1_PREFIX: str = "/api/v1"
    
    # Database
    DATABASE_URL: str = "sqlite:///./mutual_aid.db"
    
    # Initial points for new users
    INITIAL_POINTS: int = 100
    
    # Judgment settings
    MIN_JUDGES: int = 3
    JUDGMENT_PASS_THRESHOLD: float = 0.67  # 2/3 majority
    
    class Config:
        env_file = ".env"

settings = Settings()
```

- [ ] **Step 4: 创建 app/database.py**

```python
from sqlalchemy import create_engine
from sqlalchemy.ext.declarative import declarative_base
from sqlalchemy.orm import sessionmaker
from app.config import settings

engine = create_engine(
    settings.DATABASE_URL,
    connect_args={"check_same_thread": False}  # SQLite only
)

SessionLocal = sessionmaker(autocommit=False, autoflush=False, bind=engine)
Base = declarative_base()

def get_db():
    db = SessionLocal()
    try:
        yield db
    finally:
        db.close()
```

- [ ] **Step 5: 创建 tests/conftest.py**

```python
import pytest
from sqlalchemy import create_engine
from sqlalchemy.orm import sessionmaker
from app.database import Base, get_db
from app.main import app

TEST_DATABASE_URL = "sqlite:///./test_mutual_aid.db"

@pytest.fixture(scope="function")
def db_session():
    engine = create_engine(TEST_DATABASE_URL, connect_args={"check_same_thread": False})
    Base.metadata.create_all(bind=engine)
    Session = sessionmaker(bind=engine)
    session = Session()
    try:
        yield session
    finally:
        session.close()
        Base.metadata.drop_all(bind=engine)

@pytest.fixture(scope="function")
def client(db_session):
    def override_get_db():
        try:
            yield db_session
        finally:
            pass
    
    app.dependency_overrides[get_db] = override_get_db
    from fastapi.testclient import TestClient
    with TestClient(app) as c:
        yield c
    app.dependency_overrides.clear()
```

- [ ] **Step 6: 提交**

```bash
cd backend
git add -A
git commit -m "feat: initialize backend project structure"
```

---

### Task 2: User 模型和 Schema

**Files:**
- Create: `backend/app/models/user.py`
- Create: `backend/app/schemas/user.py`
- Create: `backend/tests/test_users.py`

- [ ] **Step 1: 创建 User 模型**

```python
# backend/app/models/user.py
from sqlalchemy import Column, String, Integer, DateTime
from sqlalchemy.sql import func
from app.database import Base

class User(Base):
    __tablename__ = "users"
    
    id = Column(String, primary_key=True, index=True)
    created_at = Column(DateTime(timezone=True), server_default=func.now())
    points_balance = Column(Integer, default=100)
    reputation = Column(Integer, default=100)
    tasks_completed = Column(Integer, default=0)
    tasks_posted = Column(Integer, default=0)
```

- [ ] **Step 2: 创建 User Schema**

```python
# backend/app/schemas/user.py
from pydantic import BaseModel
from datetime import datetime
from typing import Optional

class UserBase(BaseModel):
    id: str

class UserCreate(UserBase):
    pass

class UserResponse(UserBase):
    created_at: datetime
    points_balance: int
    reputation: int
    tasks_completed: int
    tasks_posted: int
    
    class Config:
        from_attributes = True

class UserPointsResponse(BaseModel):
    user_id: str
    points_balance: int
    reputation: int
```

- [ ] **Step 3: 编写 User 测试**

```python
# backend/tests/test_users.py
import pytest
from httpx import AsyncClient
from app.database import Base, engine

@pytest.mark.asyncio
async def test_register_user(client):
    """测试用户注册"""
    response = client.post("/api/v1/users", json={"id": "test-user-1"})
    assert response.status_code == 201
    data = response.json()
    assert data["id"] == "test-user-1"
    assert data["points_balance"] == 100
    assert data["reputation"] == 100

@pytest.mark.asyncio
async def test_register_duplicate_user(client):
    """测试重复注册返回 400"""
    client.post("/api/v1/users", json={"id": "test-user-2"})
    response = client.post("/api/v1/users", json={"id": "test-user-2"})
    assert response.status_code == 400

@pytest.mark.asyncio
async def test_get_user(client):
    """测试获取用户信息"""
    client.post("/api/v1/users", json={"id": "test-user-3"})
    response = client.get("/api/v1/users/test-user-3")
    assert response.status_code == 200
    data = response.json()
    assert data["id"] == "test-user-3"

@pytest.mark.asyncio
async def test_get_nonexistent_user(client):
    """测试获取不存在的用户返回 404"""
    response = client.get("/api/v1/users/nonexistent")
    assert response.status_code == 404
```

- [ ] **Step 4: 运行测试验证失败**

```bash
cd backend
pytest tests/test_users.py -v
# Expected: FAIL (API not implemented yet)
```

- [ ] **Step 5: 创建 User API 路由**

```python
# backend/app/api/users.py
from fastapi import APIRouter, Depends, HTTPException
from sqlalchemy.orm import Session
from app.database import get_db
from app.models.user import User
from app.schemas.user import UserCreate, UserResponse, UserPointsResponse

router = APIRouter(prefix="/users", tags=["users"])

@router.post("", response_model=UserResponse, status_code=201)
def register_user(user_data: UserCreate, db: Session = Depends(get_db)):
    """注册新用户"""
    existing = db.query(User).filter(User.id == user_data.id).first()
    if existing:
        raise HTTPException(status_code=400, detail="User already exists")
    
    user = User(id=user_data.id)
    db.add(user)
    db.commit()
    db.refresh(user)
    return user

@router.get("/{user_id}", response_model=UserResponse)
def get_user(user_id: str, db: Session = Depends(get_db)):
    """获取用户信息"""
    user = db.query(User).filter(User.id == user_id).first()
    if not user:
        raise HTTPException(status_code=404, detail="User not found")
    return user

@router.get("/{user_id}/points", response_model=UserPointsResponse)
def get_user_points(user_id: str, db: Session = Depends(get_db)):
    """获取用户积分"""
    user = db.query(User).filter(User.id == user_id).first()
    if not user:
        raise HTTPException(status_code=404, detail="User not found")
    return UserPointsResponse(
        user_id=user.id,
        points_balance=user.points_balance,
        reputation=user.reputation
    )
```

- [ ] **Step 6: 创建主应用入口**

```python
# backend/app/main.py
from fastapi import FastAPI
from app.config import settings
from app.database import Base, engine
from app.api import users

Base.metadata.create_all(bind=engine)

app = FastAPI(title=settings.PROJECT_NAME, version=settings.VERSION)

app.include_router(users.router, prefix=settings.API_V1_PREFIX)

@app.get("/health")
def health_check():
    return {"status": "ok"}
```

- [ ] **Step 7: 运行测试验证通过**

```bash
cd backend
pytest tests/test_users.py -v
# Expected: All tests PASS
```

- [ ] **Step 8: 提交**

```bash
cd backend
git add -A
git commit -m "feat: implement user registration and retrieval"
```

---

### Task 3: 积分服务 + Transaction 模型

**Files:**
- Create: `backend/app/models/transaction.py`
- Create: `backend/app/schemas/transaction.py`
- Create: `backend/app/services/points_service.py`
- Create: `backend/tests/test_points.py`

- [ ] **Step 1: 创建 Transaction 模型**

```python
# backend/app/models/transaction.py
from sqlalchemy import Column, String, Integer, DateTime, ForeignKey
from sqlalchemy.sql import func
from app.database import Base

class Transaction(Base):
    __tablename__ = "transactions"
    
    id = Column(String, primary_key=True, index=True)
    from_user = Column(String, ForeignKey("users.id"), nullable=True)
    to_user = Column(String, ForeignKey("users.id"))
    amount = Column(Integer)
    reason = Column(String)
    task_id = Column(String, nullable=True)
    created_at = Column(DateTime(timezone=True), server_default=func.now())
```

- [ ] **Step 2: 创建 Transaction Schema**

```python
# backend/app/schemas/transaction.py
from pydantic import BaseModel
from datetime import datetime
from typing import Optional

class TransactionBase(BaseModel):
    from_user: Optional[str] = None
    to_user: str
    amount: int
    reason: str
    task_id: Optional[str] = None

class TransactionResponse(TransactionBase):
    id: str
    created_at: datetime
    
    class Config:
        from_attributes = True
```

- [ ] **Step 3: 创建积分服务**

```python
# backend/app/services/points_service.py
import uuid
from sqlalchemy.orm import Session
from app.models.user import User
from app.models.transaction import Transaction

class PointsService:
    def __init__(self, db: Session):
        self.db = db
    
    def transfer_points(self, from_user_id: str, to_user_id: str, 
                        amount: int, reason: str, task_id: str = None):
        """转移积分（带托管验证）"""
        from_user = self.db.query(User).filter(User.id == from_user_id).first()
        to_user = self.db.query(User).filter(User.id == to_user_id).first()
        
        if not from_user:
            raise ValueError(f"User {from_user_id} not found")
        if not to_user:
            raise ValueError(f"User {to_user_id} not found")
        if from_user.points_balance < amount:
            raise ValueError("Insufficient points balance")
        
        # 执行转账
        from_user.points_balance -= amount
        to_user.points_balance += amount
        
        # 创建交易记录
        transaction = Transaction(
            id=str(uuid.uuid4()),
            from_user=from_user_id,
            to_user=to_user_id,
            amount=amount,
            reason=reason,
            task_id=task_id
        )
        self.db.add(transaction)
        self.db.commit()
        
        return transaction
    
    def freeze_points(self, user_id: str, amount: int, task_id: str):
        """冻结积分（发布任务时）"""
        user = self.db.query(User).filter(User.id == user_id).first()
        if not user:
            raise ValueError(f"User {user_id} not found")
        if user.points_balance < amount:
            raise ValueError("Insufficient points balance")
        user.points_balance -= amount
        self.db.commit()
    
    def release_points(self, user_id: str, amount: int):
        """释放冻结积分（任务失败时退回）"""
        user = self.db.query(User).filter(User.id == user_id).first()
        if user:
            user.points_balance += amount
            self.db.commit()
```

- [ ] **Step 4: 编写积分测试**

```python
# backend/tests/test_points.py
import pytest
from app.models.user import User
from app.services.points_service import PointsService

def test_transfer_points(db_session):
    """测试积分转移"""
    # 创建两个用户
    db_session.add(User(id="user-1", points_balance=100))
    db_session.add(User(id="user-2", points_balance=50))
    db_session.commit()
    
    service = PointsService(db_session)
    service.transfer_points("user-1", "user-2", 20, "test transfer")
    
    user1 = db_session.query(User).filter(User.id == "user-1").first()
    user2 = db_session.query(User).filter(User.id == "user-2").first()
    
    assert user1.points_balance == 80
    assert user2.points_balance == 70

def test_insufficient_points(db_session):
    """测试积分不足"""
    db_session.add(User(id="user-3", points_balance=10))
    db_session.commit()
    
    service = PointsService(db_session)
    with pytest.raises(ValueError, match="Insufficient"):
        service.transfer_points("user-3", "user-4", 100, "test")
```

- [ ] **Step 5: 运行测试并修复**

```bash
cd backend
pytest tests/test_points.py -v
# Expected: PASS after implementation
```

- [ ] **Step 6: 提交**

```bash
git add -A
git commit -m "feat: implement points transfer service"
```

---

## Chunk 2: 任务模块 + 裁决模块

### Task 4: Task 模型 + 任务 API

**Files:**
- Create: `backend/app/models/task.py`
- Create: `backend/app/schemas/task.py`
- Create: `backend/app/services/task_service.py`
- Create: `backend/app/api/tasks.py`
- Create: `backend/tests/test_tasks.py`

- [ ] **Step 1: 创建 Task 模型**

```python
# backend/app/models/task.py
from sqlalchemy import Column, String, Integer, DateTime, Text, ForeignKey, Enum
from sqlalchemy.sql import func
import enum
from app.database import Base

class TaskStatus(str, enum.Enum):
    PENDING = "pending"
    CLAIMED = "claimed"
    SUBMITTED = "submitted"
    COMPLETED = "completed"
    FAILED = "failed"

class Task(Base):
    __tablename__ = "tasks"
    
    id = Column(String, primary_key=True, index=True)
    publisher_id = Column(String, ForeignKey("users.id"))
    title = Column(String, index=True)
    description = Column(Text)
    goal = Column(Text)  # Immutable after creation
    reward = Column(Integer)
    status = Column(Enum(TaskStatus), default=TaskStatus.PENDING)
    created_at = Column(DateTime(timezone=True), server_default=func.now())
    claimed_by = Column(String, ForeignKey("users.id"), nullable=True)
    claimed_at = Column(DateTime(timezone=True), nullable=True)
    submitted_at = Column(DateTime(timezone=True), nullable=True)
    submission_content = Column(Text, nullable=True)
```

- [ ] **Step 2: 创建 Task Schema**

```python
# backend/app/schemas/task.py
from pydantic import BaseModel
from datetime import datetime
from typing import Optional
from app.models.task import TaskStatus

class TaskBase(BaseModel):
    title: str
    description: str
    goal: str
    reward: int

class TaskCreate(TaskBase):
    publisher_id: str

class TaskResponse(TaskBase):
    id: str
    publisher_id: str
    status: TaskStatus
    created_at: datetime
    claimed_by: Optional[str] = None
    claimed_at: Optional[datetime] = None
    submitted_at: Optional[datetime] = None
    
    class Config:
        from_attributes = True
        use_enum_values = True
```

- [ ] **Step 3: 创建 Task 服务**

```python
# backend/app/services/task_service.py
import uuid
from datetime import datetime
from sqlalchemy.orm import Session
from app.models.task import Task, TaskStatus
from app.models.user import User
from app.schemas.task import TaskCreate

class TaskService:
    def __init__(self, db: Session):
        self.db = db
    
    def create_task(self, task_data: TaskCreate) -> Task:
        """创建任务（冻结发布者积分）"""
        publisher = self.db.query(User).filter(User.id == task_data.publisher_id).first()
        if not publisher:
            raise ValueError("Publisher not found")
        if publisher.points_balance < task_data.reward:
            raise ValueError("Insufficient points for reward")
        
        # 冻结积分
        publisher.points_balance -= task_data.reward
        publisher.tasks_posted += 1
        
        task = Task(
            id=str(uuid.uuid4()),
            publisher_id=task_data.publisher_id,
            title=task_data.title,
            description=task_data.description,
            goal=task_data.goal,
            reward=task_data.reward,
            status=TaskStatus.PENDING
        )
        self.db.add(task)
        self.db.commit()
        self.db.refresh(task)
        return task
    
    def claim_task(self, task_id: str, claimer_id: str) -> Task:
        """领取任务"""
        task = self.db.query(Task).filter(Task.id == task_id).first()
        if not task:
            raise ValueError("Task not found")
        if task.status != TaskStatus.PENDING:
            raise ValueError("Task not available")
        
        task.status = TaskStatus.CLAIMED
        task.claimed_by = claimer_id
        task.claimed_at = datetime.utcnow()
        self.db.commit()
        self.db.refresh(task)
        return task
    
    def submit_task(self, task_id: str, submitter_id: str, content: str) -> Task:
        """提交任务结果"""
        task = self.db.query(Task).filter(Task.id == task_id).first()
        if not task:
            raise ValueError("Task not found")
        if task.claimed_by != submitter_id:
            raise ValueError("Not authorized to submit")
        if task.status != TaskStatus.CLAIMED:
            raise ValueError("Task not in claimed status")
        
        task.status = TaskStatus.SUBMITTED
        task.submission_content = content
        task.submitted_at = datetime.utcnow()
        self.db.commit()
        self.db.refresh(task)
        return task
    
    def complete_task(self, task_id: str):
        """完成任务（释放奖励给提交者）"""
        task = self.db.query(Task).filter(Task.id == task_id).first()
        if task:
            task.status = TaskStatus.COMPLETED
            submitter = self.db.query(User).filter(User.id == task.claimed_by).first()
            if submitter:
                submitter.points_balance += task.reward
                submitter.tasks_completed += 1
            self.db.commit()
    
    def fail_task(self, task_id: str):
        """任务失败（积分退回发布者）"""
        task = self.db.query(Task).filter(Task.id == task_id).first()
        if task:
            task.status = TaskStatus.FAILED
            publisher = self.db.query(User).filter(User.id == task.publisher_id).first()
            if publisher:
                publisher.points_balance += task.reward
            self.db.commit()
```

- [ ] **Step 4: 创建 Task API**

```python
# backend/app/api/tasks.py
from fastapi import APIRouter, Depends, HTTPException, Query
from sqlalchemy.orm import Session
from typing import List, Optional
from app.database import get_db
from app.models.task import Task, TaskStatus
from app.schemas.task import TaskCreate, TaskResponse
from app.services.task_service import TaskService

router = APIRouter(prefix="/tasks", tags=["tasks"])

@router.post("", response_model=TaskResponse, status_code=201)
def create_task(task_data: TaskCreate, db: Session = Depends(get_db)):
    service = TaskService(db)
    try:
        task = service.create_task(task_data)
        return task
    except ValueError as e:
        raise HTTPException(status_code=400, detail=str(e))

@router.get("", response_model=List[TaskResponse])
def list_tasks(
    status: Optional[TaskStatus] = None,
    limit: int = Query(20, le=100),
    offset: int = 0,
    db: Session = Depends(get_db)
):
    query = db.query(Task)
    if status:
        query = query.filter(Task.status == status)
    return query.order_by(Task.created_at.desc()).offset(offset).limit(limit).all()

@router.get("/{task_id}", response_model=TaskResponse)
def get_task(task_id: str, db: Session = Depends(get_db)):
    task = db.query(Task).filter(Task.id == task_id).first()
    if not task:
        raise HTTPException(status_code=404, detail="Task not found")
    return task

@router.post("/{task_id}/claim", response_model=TaskResponse)
def claim_task(task_id: str, claimer_id: str, db: Session = Depends(get_db)):
    service = TaskService(db)
    try:
        task = service.claim_task(task_id, claimer_id)
        return task
    except ValueError as e:
        raise HTTPException(status_code=400, detail=str(e))

@router.post("/{task_id}/submit", response_model=TaskResponse)
def submit_task(task_id: str, submitter_id: str, content: str, db: Session = Depends(get_db)):
    service = TaskService(db)
    try:
        task = service.submit_task(task_id, submitter_id, content)
        return task
    except ValueError as e:
        raise HTTPException(status_code=400, detail=str(e))
```

- [ ] **Step 5: 编写任务测试**

```python
# backend/tests/test_tasks.py
import pytest
from app.models.user import User
from app.models.task import Task, TaskStatus
from app.schemas.task import TaskCreate
from app.services.task_service import TaskService

def test_create_task(db_session):
    """测试创建任务"""
    db_session.add(User(id="publisher", points_balance=200))
    db_session.commit()
    
    service = TaskService(db_session)
    task = service.create_task(TaskCreate(
        publisher_id="publisher",
        title="Test Task",
        description="Do something",
        goal="Complete the task",
        reward=50
    ))
    
    assert task.status == TaskStatus.PENDING
    publisher = db_session.query(User).filter(User.id == "publisher").first()
    assert publisher.points_balance == 150  # 200 - 50 frozen

def test_claim_task(db_session):
    """测试领取任务"""
    db_session.add(User(id="publisher", points_balance=200))
    db_session.add(User(id="claimer", points_balance=100))
    db_session.add(Task(
        id="task-1", publisher_id="publisher",
        title="Test", description="Desc", goal="Goal", reward=50
    ))
    db_session.commit()
    
    service = TaskService(db_session)
    task = service.claim_task("task-1", "claimer")
    
    assert task.status == TaskStatus.CLAIMED
    assert task.claimed_by == "claimer"

def test_submit_task(db_session):
    """测试提交任务"""
    from datetime import datetime
    db_session.add(User(id="claimer", points_balance=100))
    db_session.add(Task(
        id="task-2", publisher_id="pub",
        title="Test", description="Desc", goal="Goal", reward=50,
        status=TaskStatus.CLAIMED, claimed_by="claimer"
    ))
    db_session.commit()
    
    service = TaskService(db_session)
    task = service.submit_task("task-2", "claimer", "Here is my work")
    
    assert task.status == TaskStatus.SUBMITTED
    assert task.submission_content == "Here is my work"
```

- [ ] **Step 6: 运行测试并修复**

```bash
cd backend
pytest tests/test_tasks.py -v
```

- [ ] **Step 7: 提交**

```bash
git add -A
git commit -m "feat: implement task CRUD and lifecycle"
```

---

### Task 5: Judgment 模型 + 裁决服务

**Files:**
- Create: `backend/app/models/judgment.py`
- Create: `backend/app/schemas/judgment.py`
- Create: `backend/app/services/judgment_service.py`
- Create: `backend/app/api/judgments.py`
- Create: `backend/tests/test_judgments.py`

- [ ] **Step 1: 创建 Judgment 模型**

```python
# backend/app/models/judgment.py
from sqlalchemy import Column, String, Boolean, DateTime, Text, ForeignKey
from sqlalchemy.sql import func
from app.database import Base

class Judgment(Base):
    __tablename__ = "judgments"
    
    id = Column(String, primary_key=True, index=True)
    task_id = Column(String, ForeignKey("tasks.id"))
    judge_id = Column(String, ForeignKey("users.id"))
    vote = Column(Boolean)  # True = pass, False = fail
    reason = Column(Text)
    created_at = Column(DateTime(timezone=True), server_default=func.now())
```

- [ ] **Step 2: 创建 Judgment Schema**

```python
# backend/app/schemas/judgment.py
from pydantic import BaseModel
from datetime import datetime

class JudgmentBase(BaseModel):
    task_id: str
    judge_id: str
    vote: bool
    reason: str

class JudgmentResponse(JudgmentBase):
    id: str
    created_at: datetime
    
    class Config:
        from_attributes = True

class JudgmentResult(BaseModel):
    task_id: str
    passed: bool
    total_votes: int
    yes_votes: int
    no_votes: int
```

- [ ] **Step 3: 创建裁决服务**

```python
# backend/app/services/judgment_service.py
import uuid
from sqlalchemy.orm import Session
from app.models.judgment import Judgment
from app.models.task import Task, TaskStatus
from app.models.user import User
from app.config import settings

class JudgmentService:
    def __init__(self, db: Session):
        self.db = db
        self.pass_threshold = settings.JUDGMENT_PASS_THRESHOLD
    
    def submit_judgment(self, task_id: str, judge_id: str, 
                        vote: bool, reason: str) -> Judgment:
        """提交裁决投票"""
        judgment = Judgment(
            id=str(uuid.uuid4()),
            task_id=task_id,
            judge_id=judge_id,
            vote=vote,
            reason=reason
        )
        self.db.add(judgment)
        self.db.commit()
        self.db.refresh(judgment)
        
        # 检查是否达到裁决条件
        self._check_judgment_complete(task_id)
        
        return judgment
    
    def _check_judgment_complete(self, task_id: str):
        """检查裁决是否完成并更新任务状态"""
        task = self.db.query(Task).filter(Task.id == task_id).first()
        if not task or task.status != TaskStatus.SUBMITTED:
            return
        
        judgments = self.db.query(Judgment).filter(
            Judgment.task_id == task_id
        ).all()
        
        if len(judgments) >= settings.MIN_JUDGES:
            yes_votes = sum(1 for j in judgments if j.vote)
            pass_rate = yes_votes / len(judgments)
            
            if pass_rate >= self.pass_threshold:
                # 任务通过
                from app.services.task_service import TaskService
                task_service = TaskService(self.db)
                task_service.complete_task(task_id)
            else:
                # 任务失败
                from app.services.task_service import TaskService
                task_service = TaskService(self.db)
                task_service.fail_task(task_id)
    
    def get_judgment_result(self, task_id: str) -> dict:
        """获取裁决结果"""
        judgments = self.db.query(Judgment).filter(
            Judgment.task_id == task_id
        ).all()
        
        yes_votes = sum(1 for j in judgments if j.vote)
        no_votes = len(judgments) - yes_votes
        
        return {
            "task_id": task_id,
            "total_votes": len(judgments),
            "yes_votes": yes_votes,
            "no_votes": no_votes
        }
```

- [ ] **Step 4: 创建 Judgment API**

```python
# backend/app/api/judgments.py
from fastapi import APIRouter, Depends, HTTPException
from sqlalchemy.orm import Session
from app.database import get_db
from app.schemas.judgment import JudgmentBase, JudgmentResponse, JudgmentResult
from app.services.judgment_service import JudgmentService

router = APIRouter(prefix="/judgments", tags=["judgments"])

@router.post("", response_model=JudgmentResponse, status_code=201)
def submit_judgment(judgment_data: JudgmentBase, db: Session = Depends(get_db)):
    service = JudgmentService(db)
    try:
        judgment = service.submit_judgment(
            judgment_data.task_id,
            judgment_data.judge_id,
            judgment_data.vote,
            judgment_data.reason
        )
        return judgment
    except ValueError as e:
        raise HTTPException(status_code=400, detail=str(e))

@router.get("/tasks/{task_id}/result", response_model=JudgmentResult)
def get_judgment_result(task_id: str, db: Session = Depends(get_db)):
    service = JudgmentService(db)
    return service.get_judgment_result(task_id)
```

- [ ] **Step 5: 编写裁决测试**

```python
# backend/tests/test_judgments.py
import pytest
from app.models.user import User
from app.models.task import Task, TaskStatus
from app.models.judgment import Judgment
from app.services.judgment_service import JudgmentService

def test_judgment_pass(db_session):
    """测试裁决通过（3 票全过）"""
    db_session.add(User(id="publisher", points_balance=100))
    db_session.add(User(id="claimer", points_balance=50))
    db_session.add(Task(
        id="task-j1", publisher_id="publisher",
        title="Test", description="Desc", goal="Goal", reward=50,
        status=TaskStatus.SUBMITTED, claimed_by="claimer"
    ))
    db_session.commit()
    
    service = JudgmentService(db_session)
    # 3 个裁决者都投赞成票
    for i in range(3):
        service.submit_judgment("task-j1", f"judge-{i}", True, "Good work")
    
    task = db_session.query(Task).filter(Task.id == "task-j1").first()
    assert task.status == TaskStatus.COMPLETED
    claimer = db_session.query(User).filter(User.id == "claimer").first()
    assert claimer.points_balance == 100  # 50 + 50 reward

def test_judgment_fail(db_session):
    """测试裁决失败（3 票中 2 票反对）"""
    db_session.add(User(id="publisher", points_balance=50))
    db_session.add(User(id="claimer", points_balance=50))
    db_session.add(Task(
        id="task-j2", publisher_id="publisher",
        title="Test", description="Desc", goal="Goal", reward=50,
        status=TaskStatus.SUBMITTED, claimed_by="claimer"
    ))
    db_session.commit()
    
    service = JudgmentService(db_session)
    service.submit_judgment("task-j2", "judge-1", False, "Not good")
    service.submit_judgment("task-j2", "judge-2", False, "Incomplete")
    service.submit_judgment("task-j2", "judge-3", True, "OK")
    
    task = db_session.query(Task).filter(Task.id == "task-j2").first()
    assert task.status == TaskStatus.FAILED
    publisher = db_session.query(User).filter(User.id == "publisher").first()
    assert publisher.points_balance == 100  # 50 + 50 refunded
```

- [ ] **Step 6: 运行测试并修复**

```bash
cd backend
pytest tests/test_judgments.py -v
```

- [ ] **Step 7: 提交**

```bash
git add -A
git commit -m "feat: implement judgment voting system"
```

---

## Chunk 3: 前端界面

### Task 6: React 项目初始化

**Files:**
- Create: `frontend/package.json`
- Create: `frontend/tsconfig.json`
- Create: `frontend/src/index.tsx`
- Create: `frontend/src/App.tsx`
- Create: `frontend/public/index.html`

- [ ] **Step 1: 创建 package.json**

```json
{
  "name": "mutual-aid-frontend",
  "version": "0.1.0",
  "private": true,
  "dependencies": {
    "react": "^18.2.0",
    "react-dom": "^18.2.0",
    "react-router-dom": "^6.21.0",
    "axios": "^1.6.5"
  },
  "devDependencies": {
    "@types/react": "^18.2.47",
    "@types/react-dom": "^18.2.18",
    "typescript": "^5.3.3",
    "vite": "^5.0.11",
    "@vitejs/plugin-react": "^4.2.1"
  },
  "scripts": {
    "dev": "vite",
    "build": "tsc && vite build",
    "preview": "vite preview"
  }
}
```

- [ ] **Step 2: 创建 tsconfig.json**

```json
{
  "compilerOptions": {
    "target": "ES2020",
    "useDefineForClassFields": true,
    "lib": ["ES2020", "DOM", "DOM.Iterable"],
    "module": "ESNext",
    "skipLibCheck": true,
    "moduleResolution": "bundler",
    "allowImportingTsExtensions": true,
    "resolveJsonModule": true,
    "isolatedModules": true,
    "noEmit": true,
    "jsx": "react-jsx",
    "strict": true,
    "noUnusedLocals": true,
    "noUnusedParameters": true,
    "noFallthroughCasesInSwitch": true
  },
  "include": ["src"],
  "references": [{ "path": "./tsconfig.node.json" }]
}
```

- [ ] **Step 3: 创建 Vite 配置**

```javascript
// frontend/vite.config.js
import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

export default defineConfig({
  plugins: [react()],
  server: {
    proxy: {
      '/api': 'http://localhost:8000'
    }
  }
})
```

- [ ] **Step 4: 创建 index.html**

```html
<!DOCTYPE html>
<html lang="zh-CN">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>OpenClaw 互助平台</title>
  </head>
  <body>
    <div id="root"></div>
    <script type="module" src="/src/index.tsx"></script>
  </body>
</html>
```

- [ ] **Step 5: 创建入口文件**

```tsx
// frontend/src/index.tsx
import React from 'react'
import ReactDOM from 'react-dom/client'
import { BrowserRouter } from 'react-router-dom'
import App from './App'
import './index.css'

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <BrowserRouter>
      <App />
    </BrowserRouter>
  </React.StrictMode>
)
```

- [ ] **Step 6: 创建 App 组件**

```tsx
// frontend/src/App.tsx
import { Routes, Route, Link } from 'react-router-dom'
import Home from './pages/Home'
import Tasks from './pages/Tasks'
import CreateTask from './pages/CreateTask'
import Profile from './pages/Profile'

function App() {
  return (
    <div className="app">
      <nav className="navbar">
        <Link to="/">首页</Link>
        <Link to="/tasks">任务列表</Link>
        <Link to="/create">发布任务</Link>
        <Link to="/profile">个人中心</Link>
      </nav>
      <main className="main-content">
        <Routes>
          <Route path="/" element={<Home />} />
          <Route path="/tasks" element={<Tasks />} />
          <Route path="/create" element={<CreateTask />} />
          <Route path="/profile/:userId" element={<Profile />} />
        </Routes>
      </main>
    </div>
  )
}

export default App
```

- [ ] **Step 7: 安装依赖并测试**

```bash
cd frontend
npm install
npm run dev
# Verify: App renders without errors
```

- [ ] **Step 8: 提交**

```bash
git add -A
git commit -m "feat: initialize React frontend project"
```

---

### Task 7: 任务列表页面

**Files:**
- Create: `frontend/src/pages/Tasks.tsx`
- Create: `frontend/src/components/TaskList.tsx`
- Create: `frontend/src/api/client.ts`
- Create: `frontend/src/types/index.ts`

- [ ] **Step 1: 创建类型定义**

```typescript
// frontend/src/types/index.ts
export interface User {
  id: string;
  created_at: string;
  points_balance: number;
  reputation: number;
  tasks_completed: number;
  tasks_posted: number;
}

export interface Task {
  id: string;
  publisher_id: string;
  title: string;
  description: string;
  goal: string;
  reward: number;
  status: 'pending' | 'claimed' | 'submitted' | 'completed' | 'failed';
  created_at: string;
  claimed_by?: string;
  claimed_at?: string;
  submitted_at?: string;
}

export interface Judgment {
  id: string;
  task_id: string;
  judge_id: string;
  vote: boolean;
  reason: string;
  created_at: string;
}
```

- [ ] **Step 2: 创建 API 客户端**

```typescript
// frontend/src/api/client.ts
import axios from 'axios'

const api = axios.create({
  baseURL: '/api/v1'
})

export const taskApi = {
  list: (status?: string) => 
    api.get<Task[]>('/tasks', { params: { status } }),
  get: (id: string) => 
    api.get<Task>(`/tasks/${id}`),
  create: (data: { publisher_id: string; title: string; description: string; goal: string; reward: number }) =>
    api.post<Task>('/tasks', data),
  claim: (taskId: string, claimerId: string) =>
    api.post<Task>(`/tasks/${taskId}/claim`, { claimer_id: claimerId }),
  submit: (taskId: string, submitterId: string, content: string) =>
    api.post<Task>(`/tasks/${taskId}/submit`, { submitter_id: submitterId, content })
}

export const userApi = {
  register: (id: string) =>
    api.post<User>('/users', { id }),
  get: (id: string) =>
    api.get<User>(`/users/${id}`),
  getPoints: (id: string) =>
    api.get<{ user_id: string; points_balance: number; reputation: number }>(`/users/${id}/points`)
}

export const judgmentApi = {
  submit: (data: { task_id: string; judge_id: string; vote: boolean; reason: string }) =>
    api.post('/judgments', data),
  getResult: (taskId: string) =>
    api.get(`/judgments/tasks/${taskId}/result`)
}
```

- [ ] **Step 3: 创建任务列表组件**

```tsx
// frontend/src/components/TaskList.tsx
import React from 'react'
import { Task } from '../types'

interface TaskListProps {
  tasks: Task[]
  onClaim?: (taskId: string) => void
  userId?: string
}

const statusLabels: Record<string, string> = {
  pending: '待领取',
  claimed: '进行中',
  submitted: '待裁决',
  completed: '已完成',
  failed: '已失败'
}

const statusColors: Record<string, string> = {
  pending: '#1890ff',
  claimed: '#faad14',
  submitted: '#722ed1',
  completed: '#52c41a',
  failed: '#ff4d4f'
}

export const TaskList: React.FC<TaskListProps> = ({ tasks, onClaim, userId }) => {
  return (
    <div className="task-list">
      {tasks.map(task => (
        <div key={task.id} className="task-card">
          <h3>{task.title}</h3>
          <p>{task.description}</p>
          <div className="task-meta">
            <span className="status" style={{ color: statusColors[task.status] }}>
              {statusLabels[task.status]}
            </span>
            <span className="reward">🪙 {task.reward} 积分</span>
          </div>
          {onClaim && task.status === 'pending' && userId && (
            <button onClick={() => onClaim(task.id)}>领取任务</button>
          )}
        </div>
      ))}
    </div>
  )
}
```

- [ ] **Step 4: 创建任务列表页面**

```tsx
// frontend/src/pages/Tasks.tsx
import React, { useState, useEffect } from 'react'
import { useSearchParams } from 'react-router-dom'
import { TaskList } from '../components/TaskList'
import { taskApi } from '../api/client'
import { Task } from '../types'

const Tasks: React.FC = () => {
  const [tasks, setTasks] = useState<Task[]>([])
  const [searchParams] = useSearchParams()
  const status = searchParams.get('status') || undefined
  const userId = localStorage.getItem('userId') || undefined

  useEffect(() => {
    taskApi.list(status).then(res => setTasks(res.data))
  }, [status])

  const handleClaim = async (taskId: string) => {
    if (!userId) {
      alert('请先登录')
      return
    }
    try {
      await taskApi.claim(taskId, userId)
      alert('任务领取成功')
      window.location.reload()
    } catch (e) {
      alert('领取失败：' + (e as any).response?.data?.detail)
    }
  }

  return (
    <div>
      <h1>任务列表</h1>
      <div className="filters">
        <a href="/tasks">全部</a>
        <a href="/tasks?status=pending">待领取</a>
        <a href="/tasks?status=claimed">进行中</a>
        <a href="/tasks?status=submitted">待裁决</a>
      </div>
      <TaskList tasks={tasks} onClaim={handleClaim} userId={userId} />
    </div>
  )
}

export default Tasks
```

- [ ] **Step 5: 添加基础样式**

```css
/* frontend/src/index.css */
* { box-sizing: border-box; margin: 0; padding: 0; }
body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; }
.app { max-width: 1200px; margin: 0 auto; padding: 20px; }
.navbar { display: flex; gap: 20px; margin-bottom: 20px; padding-bottom: 20px; border-bottom: 1px solid #eee; }
.navbar a { text-decoration: none; color: #1890ff; }
.task-list { display: grid; gap: 16px; }
.task-card { border: 1px solid #eee; border-radius: 8px; padding: 16px; }
.task-meta { display: flex; justify-content: space-between; margin: 12px 0; }
.filters { display: flex; gap: 12px; margin-bottom: 16px; }
.filters a { color: #666; }
```

- [ ] **Step 6: 测试运行**

```bash
cd frontend
npm run dev
# Verify: Task list page renders and fetches data
```

- [ ] **Step 7: 提交**

```bash
git add -A
git commit -m "feat: implement task list page"
```

---

### Task 8: 任务发布和详情页面

**Files:**
- Create: `frontend/src/pages/CreateTask.tsx`
- Create: `frontend/src/pages/TaskDetail.tsx`
- Create: `frontend/src/components/TaskForm.tsx`

- [ ] **Step 1: 创建任务表单组件**

```tsx
// frontend/src/components/TaskForm.tsx
import React, { useState } from 'react'

interface TaskFormProps {
  onSubmit: (data: { title: string; description: string; goal: string; reward: number }) => void
}

export const TaskForm: React.FC<TaskFormProps> = ({ onSubmit }) => {
  const [title, setTitle] = useState('')
  const [description, setDescription] = useState('')
  const [goal, setGoal] = useState('')
  const [reward, setReward] = useState(50)

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault()
    onSubmit({ title, description, goal, reward })
  }

  return (
    <form onSubmit={handleSubmit} className="task-form">
      <div className="form-group">
        <label>任务标题</label>
        <input value={title} onChange={e => setTitle(e.target.value)} required />
      </div>
      <div className="form-group">
        <label>任务描述</label>
        <textarea value={description} onChange={e => setDescription(e.target.value)} required />
      </div>
      <div className="form-group">
        <label>任务目标（不可修改）</label>
        <textarea value={goal} onChange={e => setGoal(e.target.value)} required />
      </div>
      <div className="form-group">
        <label>奖励积分</label>
        <input type="number" value={reward} onChange={e => setReward(Number(e.target.value))} min="1" required />
      </div>
      <button type="submit">发布任务</button>
    </form>
  )
}
```

- [ ] **Step 2: 创建发布任务页面**

```tsx
// frontend/src/pages/CreateTask.tsx
import React from 'react'
import { useNavigate } from 'react-router-dom'
import { TaskForm } from '../components/TaskForm'
import { taskApi } from '../api/client'

const CreateTask: React.FC = () => {
  const navigate = useNavigate()
  const userId = localStorage.getItem('userId')

  const handleSubmit = async (data: { title: string; description: string; goal: string; reward: number }) => {
    if (!userId) {
      alert('请先登录')
      return
    }
    try {
      await taskApi.create({ ...data, publisher_id: userId })
      alert('任务发布成功')
      navigate('/tasks')
    } catch (e) {
      alert('发布失败：' + (e as any).response?.data?.detail)
    }
  }

  return (
    <div>
      <h1>发布任务</h1>
      <TaskForm onSubmit={handleSubmit} />
    </div>
  )
}

export default CreateTask
```

- [ ] **Step 3: 创建任务详情页面**

```tsx
// frontend/src/pages/TaskDetail.tsx
import React, { useState, useEffect } from 'react'
import { useParams, useNavigate } from 'react-router-dom'
import { taskApi, judgmentApi } from '../api/client'
import { Task, Judgment } from '../types'

const TaskDetail: React.FC = () => {
  const { taskId } = useParams<{ taskId: string }>()
  const navigate = useNavigate()
  const [task, setTask] = useState<Task | null>(null)
  const [judgments, setJudgments] = useState<Judgment[]>([])
  const [vote, setVote] = useState<boolean | null>(null)
  const [reason, setReason] = useState('')
  const userId = localStorage.getItem('userId') || ''

  useEffect(() => {
    if (taskId) {
      taskApi.get(taskId).then(res => setTask(res.data))
    }
  }, [taskId])

  const handleSubmitJudgment = async () => {
    if (!taskId || vote === null) return
    try {
      await judgmentApi.submit({
        task_id: taskId,
        judge_id: userId,
        vote,
        reason
      })
      alert('裁决已提交')
      navigate('/tasks')
    } catch (e) {
      alert('提交失败')
    }
  }

  if (!task) return <div>加载中...</div>

  return (
    <div className="task-detail">
      <h1>{task.title}</h1>
      <p>{task.description}</p>
      <div className="task-info">
        <p><strong>目标:</strong> {task.goal}</p>
        <p><strong>奖励:</strong> {task.reward} 积分</p>
        <p><strong>状态:</strong> {task.status}</p>
        {task.submission_content && (
          <div className="submission">
            <h3>提交内容</h3>
            <pre>{task.submission_content}</pre>
          </div>
        )}
      </div>
      {task.status === 'submitted' && userId && (
        <div className="judgment-panel">
          <h3>裁决任务</h3>
          <button onClick={() => setVote(true)}>✅ 通过</button>
          <button onClick={() => setVote(false)}>❌ 不通过</button>
          <textarea value={reason} onChange={e => setReason(e.target.value)} placeholder="裁决理由" />
          <button onClick={handleSubmitJudgment}>提交裁决</button>
        </div>
      )}
    </div>
  )
}

export default TaskDetail
```

- [ ] **Step 4: 添加路由**

```tsx
// 更新 frontend/src/App.tsx
import TaskDetail from './pages/TaskDetail'

// 在 Routes 中添加
<Route path="/tasks/:taskId" element={<TaskDetail />} />
```

- [ ] **Step 5: 提交**

```bash
git add -A
git commit -m "feat: implement create task and task detail pages"
```

---

### Task 9: 个人中心页面

**Files:**
- Create: `frontend/src/pages/Profile.tsx`
- Create: `frontend/src/pages/Home.tsx`

- [ ] **Step 1: 创建首页**

```tsx
// frontend/src/pages/Home.tsx
import React from 'react'
import { Link } from 'react-router-dom'

const Home: React.FC = () => {
  return (
    <div className="home">
      <h1>🤝 OpenClaw 互助平台</h1>
      <p>一个去中心化的 Agent 任务协作平台</p>
      <div className="features">
        <div className="feature">
          <h3>📝 发布任务</h3>
          <p>描述你的需求，设定积分奖励</p>
        </div>
        <div className="feature">
          <h3>🎯 领取任务</h3>
          <p>找到适合的任务，赚取积分</p>
        </div>
        <div className="feature">
          <h3>⚖️ 去中心化裁决</h3>
          <p>多 Agent 投票，公平公正</p>
        </div>
      </div>
      <div className="cta">
        <Link to="/tasks" className="btn">浏览任务</Link>
        <Link to="/create" className="btn primary">发布任务</Link>
      </div>
    </div>
  )
}

export default Home
```

- [ ] **Step 2: 创建个人中心页面**

```tsx
// frontend/src/pages/Profile.tsx
import React, { useState, useEffect } from 'react'
import { useParams } from 'react-router-dom'
import { userApi } from '../api/client'

const Profile: React.FC = () => {
  const { userId } = useParams<{ userId: string }>()
  const [user, setUser] = useState<any>(null)

  useEffect(() => {
    if (userId) {
      userApi.get(userId).then(res => setUser(res.data))
    }
  }, [userId])

  const handleRegister = () => {
    const id = prompt('请输入你的 Agent ID:')
    if (id) {
      userApi.register(id).then(() => {
        localStorage.setItem('userId', id)
        window.location.reload()
      })
    }
  }

  if (!user) {
    return (
      <div>
        <h1>个人中心</h1>
        <button onClick={handleRegister}>注册/登录</button>
      </div>
    )
  }

  return (
    <div className="profile">
      <h1>{user.id}</h1>
      <div className="stats">
        <div className="stat">
          <span className="value">{user.points_balance}</span>
          <span className="label">积分</span>
        </div>
        <div className="stat">
          <span className="value">{user.reputation}</span>
          <span className="label">信誉</span>
        </div>
        <div className="stat">
          <span className="value">{user.tasks_completed}</span>
          <span className="label">完成</span>
        </div>
        <div className="stat">
          <span className="value">{user.tasks_posted}</span>
          <span className="label">发布</span>
        </div>
      </div>
    </div>
  )
}

export default Profile
```

- [ ] **Step 3: 测试运行**

```bash
cd frontend
npm run dev
# Verify: All pages render correctly
```

- [ ] **Step 4: 提交**

```bash
git add -A
git commit -m "feat: implement home and profile pages"
```

---

## Chunk 4: OpenClaw Agent 集成 + 部署

### Task 10: OpenClaw Agent SDK

**Files:**
- Create: `backend/app/services/openclaw_service.py`
- Create: `scripts/register_agent.py`
- Create: `docs/AGENT_INTEGRATION.md`

- [ ] **Step 1: 创建 OpenClaw 服务**

```python
# backend/app/services/openclaw_service.py
import httpx
from typing import List, Dict, Optional

class OpenClawService:
    """OpenClaw Agent 集成服务"""
    
    def __init__(self, gateway_url: str = "http://localhost:8080"):
        self.gateway_url = gateway_url
    
    async def get_session_id(self) -> Optional[str]:
        """获取当前 Session ID"""
        # 通过 OpenClaw API 获取
        pass
    
    async def notify_task_available(self, task_id: str, agent_ids: List[str]):
        """通知 Agent 有新任务"""
        for agent_id in agent_ids:
            # 发送通知到 Agent
            pass
    
    async def notify_judgment_needed(self, task_id: str, judge_ids: List[str]):
        """通知裁决 Agent 需要裁决"""
        for judge_id in judge_ids:
            # 发送裁决请求
            pass
    
    async def get_available_agents(self) -> List[Dict]:
        """获取可用 Agent 列表"""
        # 查询 OpenClaw 注册的 Agents
        return []
```

- [ ] **Step 2: 创建 Agent 注册脚本**

```python
# scripts/register_agent.py
#!/usr/bin/env python3
"""Agent 注册到互助平台"""

import requests
import sys

PLATFORM_URL = "http://localhost:8000/api/v1"

def register_agent(agent_id: str, role: str = "worker"):
    """注册 Agent"""
    # 首先注册为用户
    resp = requests.post(f"{PLATFORM_URL}/users", json={"id": agent_id})
    if resp.status_code == 201:
        print(f"✅ Agent {agent_id} 注册成功")
        print(f"   初始积分：100")
    elif resp.status_code == 400:
        print(f"ℹ️  Agent {agent_id} 已存在")
    else:
        print(f"❌ 注册失败：{resp.text}")
        sys.exit(1)

if __name__ == "__main__":
    if len(sys.argv) < 2:
        print("用法：python register_agent.py <agent_id> [role]")
        sys.exit(1)
    register_agent(sys.argv[1], sys.argv[2] if len(sys.argv) > 2 else "worker")
```

- [ ] **Step 3: 创建集成文档**

```markdown
# Agent 集成指南

## 注册 Agent

```bash
python scripts/register_agent.py my-agent-id
```

## Agent 领取任务

```python
import requests

def claim_task(task_id: str, agent_id: str):
    resp = requests.post(
        f"http://localhost:8000/api/v1/tasks/{task_id}/claim",
        json={"claimer_id": agent_id}
    )
    return resp.json()
```

## Agent 提交任务

```python
def submit_task(task_id: str, agent_id: str, content: str):
    resp = requests.post(
        f"http://localhost:8000/api/v1/tasks/{task_id}/submit",
        json={"submitter_id": agent_id, "content": content}
    )
    return resp.json()
```

## Agent 参与裁决

```python
def submit_judgment(task_id: str, judge_id: str, vote: bool, reason: str):
    resp = requests.post(
        "http://localhost:8000/api/v1/judgments",
        json={
            "task_id": task_id,
            "judge_id": judge_id,
            "vote": vote,
            "reason": reason
        }
    )
    return resp.json()
```
```

- [ ] **Step 4: 提交**

```bash
git add -A
git commit -m "feat: add OpenClaw Agent integration"
```

---

### Task 11: Docker 部署配置

**Files:**
- Create: `Dockerfile.backend`
- Create: `Dockerfile.frontend`
- Create: `docker-compose.yml`
- Create: `.env.example`

- [ ] **Step 1: 创建后端 Dockerfile**

```dockerfile
# Dockerfile.backend
FROM python:3.11-slim

WORKDIR /app
COPY backend/requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt

COPY backend/ /app/

EXPOSE 8000
CMD ["uvicorn", "app.main:app", "--host", "0.0.0.0", "--port", "8000"]
```

- [ ] **Step 2: 创建前端 Dockerfile**

```dockerfile
# Dockerfile.frontend
FROM node:20-alpine as build

WORKDIR /app
COPY frontend/package*.json ./
RUN npm install
COPY frontend/ ./
RUN npm run build

FROM nginx:alpine
COPY --from=build /app/dist /usr/share/nginx/html
COPY frontend/nginx.conf /etc/nginx/conf.d/default.conf

EXPOSE 80
```

- [ ] **Step 3: 创建 docker-compose.yml**

```yaml
version: '3.8'

services:
  backend:
    build:
      context: .
      dockerfile: Dockerfile.backend
    ports:
      - "8000:8000"
    environment:
      - DATABASE_URL=postgresql://postgres:postgres@db:5432/mutual_aid
    depends_on:
      - db
    volumes:
      - ./backend:/app

  frontend:
    build:
      context: .
      dockerfile: Dockerfile.frontend
    ports:
      - "3000:80"
    depends_on:
      - backend

  db:
    image: postgres:15
    environment:
      - POSTGRES_PASSWORD=postgres
      - POSTGRES_DB=mutual_aid
    volumes:
      - postgres_data:/var/lib/postgresql/data

volumes:
  postgres_data:
```

- [ ] **Step 4: 创建环境示例**

```bash
# .env.example
DATABASE_URL=postgresql://postgres:postgres@db:5432/mutual_aid
INITIAL_POINTS=100
MIN_JUDGES=3
JUDGMENT_PASS_THRESHOLD=0.67
```

- [ ] **Step 5: 创建 README**

```markdown
# OpenClaw 互助平台

## 快速启动

### 开发模式

```bash
# 启动后端
cd backend
uvicorn app.main:app --reload

# 启动前端
cd frontend
npm run dev
```

### Docker 部署

```bash
docker-compose up -d
```

访问 http://localhost:3000

## API 文档

启动后访问 http://localhost:8000/docs
```

- [ ] **Step 6: 提交**

```bash
git add -A
git commit -m "feat: add Docker deployment configuration"
```

---

## 验收清单

完成所有任务后，验证以下功能：

- [ ] 用户可以注册并获得 100 初始积分
- [ ] 用户可以发布任务（积分被冻结）
- [ ] 其他用户可以浏览任务列表
- [ ] 用户可以领取待处理的任务
- [ ] 领取者可以提交任务结果
- [ ] 裁决者可以对提交的任务投票
- [ ] 达到 3 票后自动结算（≥2/3 通过则奖励，否则退回）
- [ ] 所有交易记录可查询
- [ ] 前端页面可以正常浏览和操作
- [ ] Docker 部署可以正常运行

---

## 后续扩展

完成基础版本后，可以考虑：

1. **WebSocket 实时通知** — 任务状态变更实时推送
2. **任务分类和标签** — 支持按类型筛选（代码/内容/数据等）
3. **信誉系统升级** — 基于历史表现的动态信誉分
4. **任务协作** — 支持多人协作完成一个任务
5. **API Webhook** — 任务状态变更通知到 Agent
6. **数据分析面板** — 任务完成率、平均完成时间统计
7. **申诉机制** — 对裁决结果不满可发起申诉

---

## 执行建议

**推荐执行方式：** 使用 `superpowers:subagent-driven-development` 并行执行 Chunk 1-4

**预计工作量：**
- Chunk 1 (后端基础 + 用户模块): ~2 小时
- Chunk 2 (任务 + 裁决模块): ~3 小时
- Chunk 3 (前端界面): ~3 小时
- Chunk 4 (集成 + 部署): ~1 小时

**总计**: ~9 小时（可并行缩减至 ~4 小时）

---

**Plan complete and saved to `docs/superpowers/plans/2026-03-25-openclaw-mutual-aid-platform-plan.md`. Ready to execute?**