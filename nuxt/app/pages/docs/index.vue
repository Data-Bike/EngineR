<template>
  <div class="p-4 sm:p-6 space-y-6 max-w-full overflow-x-hidden">
    <div>
      <h1 class="text-2xl font-bold">{{ t('nav.docs') }}</h1>
      <p class="text-[var(--ui-text-muted)] mt-1">{{ t('docs.subtitle') }}</p>
    </div>

    <div class="grid grid-cols-1 lg:grid-cols-3 gap-4">
      <!-- Doc sections sidebar -->
      <div class="lg:col-span-1 space-y-3">
        <UCard v-for="doc in docSections" :key="doc.title">
          <template #header>
            <div class="flex items-center gap-2">
              <UIcon :name="doc.icon" class="text-[var(--ui-primary)]" />
              <h3 class="font-semibold">{{ doc.title }}</h3>
            </div>
          </template>
          <ul class="space-y-1">
            <li v-for="item in doc.items" :key="item.id">
              <button
                class="text-left w-full text-[var(--ui-primary)] hover:underline flex items-center gap-2 text-sm py-1"
                @click="selectDoc(item)"
              >
                <UIcon name="i-lucide-file-text" class="text-xs shrink-0" />
                {{ item.title }}
              </button>
            </li>
          </ul>
        </UCard>
      </div>

      <!-- Content panel -->
      <div class="lg:col-span-2">
        <UCard v-if="selectedDoc">
          <template #header>
            <div class="flex items-center gap-2">
              <UIcon :name="selectedDoc.icon" class="text-[var(--ui-primary)]" />
              <h2 class="font-semibold text-lg">{{ selectedDoc.title }}</h2>
            </div>
          </template>
          <div class="prose prose-sm max-w-none space-y-3" v-html="selectedDoc.content" />
        </UCard>
        <UCard v-else>
          <div class="text-center py-12 text-[var(--ui-text-muted)]">
            <UIcon name="i-lucide-book-open" class="text-4xl mb-2" />
            <p>{{ t('docs.select_prompt') }}</p>
          </div>
        </UCard>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue';
import { useI18n } from 'vue-i18n';

definePageMeta({ middleware: 'auth' });

const { t } = useI18n();

interface DocItem {
  id: string;
  title: string;
  icon: string;
  content: string;
}

const selectedDoc = ref<DocItem | null>(null);

function selectDoc(item: DocItem) {
  selectedDoc.value = item;
}

const docSections: { title: string; icon: string; items: DocItem[] }[] = [
  {
    title: 'Руководство пользователя',
    icon: 'i-lucide-book-open',
    items: [
      {
        id: 'quick-start',
        title: 'Быстрый старт',
        icon: 'i-lucide-book-open',
        content: `
          <h3>Быстрый старт</h3>
          <p>EngineR — платформа управления комплаенс-данными контрагентов.</p>
          <h4>Основные разделы</h4>
          <ul>
            <li><strong>Объекты</strong> — реестр физических и юридических лиц. Создавайте карточки, заполняйте реквизиты, связывайте между собой.</li>
            <li><strong>Заявки</strong> — заявки на проверку контрагента. Создайте заявку, укажите продукт и срок — система запустит проверки.</li>
            <li><strong>Проверки</strong> — санкционные списки, PEP, негативные СМИ, реестры. Каждая проверка имеет статус и доказательства.</li>
            <li><strong>Риски</strong> — оценки риска с факторами, решениями и обоснованиями.</li>
            <li><strong>Мониторинг</strong> — регулярные проверки изменений по контрагентам.</li>
            <li><strong>Отчёты</strong> — генерация отчётов в форматах XLSX, CSV, PDF.</li>
          </ul>
          <h4>Поиск</h4>
          <p>На странице «Объекты» используйте строку поиска для поиска по названию, ИНН, ОГРН и другим полям. Расширенный поиск позволяет фильтровать по типу и полям.</p>
        `,
      },
      {
        id: 'work-with-objects',
        title: 'Работа с объектами',
        icon: 'i-lucide-book-open',
        content: `
          <h3>Работа с объектами</h3>
          <h4>Создание объекта</h4>
          <ol>
            <li>Перейдите в раздел «Объекты»</li>
            <li>Выберите тип объекта (Физическое лицо, Юридическое лицо)</li>
            <li>Нажмите кнопку «Создать»</li>
            <li>Заполните реквизиты: для ФЛ — ФИО, дата рождения, ИНН; для ЮЛ — наименование, ИНН, ОГРН, КПП</li>
            <li>Нажмите «Сохранить»</li>
          </ol>
          <h4>Карточка объекта</h4>
          <p>Карточка содержит 8 вкладок:</p>
          <ul>
            <li><strong>Обзор</strong> — основные данные объекта</li>
            <li><strong>Связи</strong> — отношения с другими объектами (управление, владение, адреса, контакты)</li>
            <li><strong>Граф</strong> — визуализация связей в виде дерева</li>
            <li><strong>Проверки</strong> — список проверок по объекту</li>
            <li><strong>Риски</strong> — оценки риска и факторы</li>
            <li><strong>Процессы</strong> — задачи и workflow</li>
            <li><strong>Документы</strong> — прикреплённые документы</li>
            <li><strong>История</strong> — журнал изменений с возможностью сравнения редакций</li>
          </ul>
          <h4>Добавление связей</h4>
          <p>На вкладке «Связи» нажмите «Добавить» для создания нового связанного объекта или «Связать существующий» для поиска и привязки уже существующего объекта.</p>
        `,
      },
      {
        id: 'create-requests',
        title: 'Создание заявок на проверку',
        icon: 'i-lucide-book-open',
        content: `
          <h3>Создание заявок на проверку</h3>
          <ol>
            <li>Перейдите в раздел «Заявки»</li>
            <li>Нажмите «Новая заявка»</li>
            <li>Выберите целевой объект (контрагента)</li>
            <li>Укажите запрошенный продукт (например, кредитный лимит)</li>
            <li>Выберите срочность (низкая, средняя, высокая, критичная)</li>
            <li>Укажите срок исполнения (опционально)</li>
            <li>Нажмите «Создать»</li>
          </ol>
          <p>После создания заявки система автоматически запустит необходимые проверки в зависимости от продукта и типа контрагента.</p>
        `,
      },
      {
        id: 'view-checks',
        title: 'Просмотр результатов проверок',
        icon: 'i-lucide-book-open',
        content: `
          <h3>Просмотр результатов проверок</h3>
          <p>Раздел «Проверки» содержит все проверки по всем объектам. Каждая проверка имеет:</p>
          <ul>
            <li><strong>Тип</strong> — санкционные списки, PEP, негативные СМИ, реестры, идентификация, адрес, налоговая</li>
            <li><strong>Статус</strong> — в очереди, выполняется, завершено, ошибка, пропущено, отменено</li>
            <li><strong>Доказательства</strong> — источники данных, подтверждающие результат</li>
            <li><strong>Коннектор</strong> — источник данных, через который выполнена проверка</li>
          </ul>
          <p>На карточке объекта перейдите на вкладку «Проверки» для просмотра проверок по конкретному объекту. Нажмите «Запустить проверку» для инициации новой проверки.</p>
        `,
      },
      {
        id: 'work-with-risks',
        title: 'Работа с рисками и решениями',
        icon: 'i-lucide-book-open',
        content: `
          <h3>Работа с рисками и решениями</h3>
          <p>Раздел «Риски» содержит оценки риска по объектам. Каждая оценка включает:</p>
          <ul>
            <li><strong>Уровень риска</strong> — минимальный, низкий, средний, высокий, критичный</li>
            <li><strong>Факторы риска</strong> — критерии, повлиявшие на оценку, с уровнем серьёзности</li>
            <li><strong>Решение</strong> — принять, отклонить, эскалировать, отложить, условное одобрение</li>
            <li><strong>Обоснование</strong> — текстовое описание причин решения</li>
            <li><strong>Статус согласования</strong> — ожидает согласования / согласовано</li>
          </ul>
          <p>Для создания новой оценки перейдите на вкладку «Риски» в карточке объекта и нажмите «Новая оценка».</p>
        `,
      },
      {
        id: 'generate-reports',
        title: 'Генерация отчётов',
        icon: 'i-lucide-book-open',
        content: `
          <h3>Генерация отчётов</h3>
          <ol>
            <li>Перейдите в раздел «Отчёты»</li>
            <li>Нажмите «Сгенерировать»</li>
            <li>Выберите объект (контрагента)</li>
            <li>Выберите шаблон отчёта</li>
            <li>Выберите формат (XLSX, CSV, JSON, DOCX, PDF)</li>
            <li>Нажмите «Сгенерировать»</li>
          </ol>
          <p>После генерации отчёт появится в списке со статусом. Готовые отчёты можно скачать.</p>
        `,
      },
    ],
  },
  {
    title: 'Руководство администратора',
    icon: 'i-lucide-settings',
    items: [
      {
        id: 'schema-management',
        title: 'Управление схемой данных',
        icon: 'i-lucide-settings',
        content: `
          <h3>Управление схемой данных</h3>
          <p>Раздел «Администрирование → Управление схемой» позволяет управлять структурой данных.</p>
          <h4>Типы объектов</h4>
          <p>Типы объектов определяют структуру сущностей: ФЛ, ЮЛ, адрес, контакт, документ и др. Каждый тип имеет набор полей (предикатов) и связей.</p>
          <h4>Черновики и публикация</h4>
          <p>Изменения схемы выполняются через черновики:</p>
          <ol>
            <li>Создайте черновик</li>
            <li>Внесите изменения (новые типы, поля, связи)</li>
            <li>Проверьте (validate) — структура корректна</li>
            <li>Оцените влияние (impact) — сколько объектов затронуто</li>
            <li>Опубликуйте (publish) — изменения вступают в силу</li>
          </ol>
          <h4>Связи и направление</h4>
          <p>Связи между объектами имеют направление. Например, связь «управляет» от ЮЛ к ФЛ означает, что ЮЛ управляет ФЛ. В карточке ФЛ эта связь отображается как входящая («управляется ЮЛ»), в карточке ЮЛ — как исходящая («управляет ФЛ»).</p>
          <h4>Права доступа</h4>
          <p>Права доступа настраиваются через раздел «Администрирование → Типы и связи». Для каждого типа и связи можно настроить права на чтение, создание, изменение, удаление по группам пользователей.</p>
        `,
      },
      {
        id: 'connector-management',
        title: 'Управление коннекторами',
        icon: 'i-lucide-settings',
        content: `
          <h3>Управление коннекторами</h3>
          <p>Раздел «Интеграции» позволяет настраивать источники данных (коннекторы).</p>
          <h4>Создание коннектора</h4>
          <ol>
            <li>Нажмите «Новый коннектор»</li>
            <li>Выберите тип коннектора (API, файловый импорт, ручной ввод)</li>
            <li>Укажите окружение (production, staging, test)</li>
            <li>Настройте rate limit (запросов в секунду)</li>
            <li>Укажите уровень доверия (Untrusted, Low, Medium, High, Authoritative)</li>
          </ol>
          <p>Коннекторы можно включать/выключать и проверять их здоровье (health check).</p>
        `,
      },
      {
        id: 'security-policies',
        title: 'Настройка политик безопасности',
        icon: 'i-lucide-settings',
        content: `
          <h3>Настройка политик безопасности</h3>
          <p>EngineR поддерживает два класса прав:</p>
          <ul>
            <li><strong>Системные capabilities</strong> — глобальные права (admin.view, entity.create, check.run и т.д.)</li>
            <li><strong>Data permissions</strong> — права на данные (тип, объект, поле, связь, классификация)</li>
          </ul>
          <h4>Права на связи</h4>
          <p>Для каждого типа связи можно настроить права по направлению: например, разрешить просмотр исходящих связей «управляет», но запретить входящих.</p>
          <h4>Маскирование полей</h4>
          <p>Поля могут быть скрыты, замаскированы полностью или частично, токенизированы в зависимости от прав пользователя.</p>
        `,
      },
      {
        id: 'user-management',
        title: 'Управление пользователями',
        icon: 'i-lucide-settings',
        content: `
          <h3>Управление пользователями</h3>
          <p>Управление пользователями включает:</p>
          <ul>
            <li>Создание и редактирование учётных записей</li>
            <li>Назначение групп и ролей</li>
            <li>Настройка MFA</li>
            <li>Просмотр активных сессий и их отзыв</li>
            <li>Аудит действий пользователя</li>
          </ul>
        `,
      },
      {
        id: 'audit-system',
        title: 'Аудит системы',
        icon: 'i-lucide-settings',
        content: `
          <h3>Аудит системы</h3>
          <p>Раздел «Аудит» содержит журнал всех действий в системе. Записи включают:</p>
          <ul>
            <li>Время действия</li>
            <li>Тип действия (создание, изменение, удаление)</li>
            <li>Инициатора (пользователь или система)</li>
            <li>Ресурс (тип и ID объекта)</li>
            <li>Результат (успех/отказ)</li>
          </ul>
          <p>Журнал защищён от изменений (hash chain). Доступны фильтры по типу, инициатору, ресурсу и периоду.</p>
        `,
      },
      {
        id: 'monitoring-setup',
        title: 'Настройка мониторинга',
        icon: 'i-lucide-settings',
        content: `
          <h3>Настройка мониторинга</h3>
          <p>Раздел «Мониторинг» позволяет настраивать регулярные проверки изменений по контрагентам.</p>
          <ul>
            <li><strong>Частота</strong> — ежедневно, еженедельно, ежемесячно, ежеквартально, ежегодно</li>
            <li><strong>Объект мониторинга</strong> — конкретный контрагент или группа</li>
            <li><strong>Тип проверки</strong> — санкционные списки, реестры, негативные СМИ</li>
          </ul>
          <p>При обнаружении изменений система создаёт уведомление и может автоматически запустить новую проверку.</p>
        `,
      },
    ],
  },
  {
    title: 'API документация',
    icon: 'i-lucide-code',
    items: [
      {
        id: 'rest-api-v1',
        title: 'REST API v1',
        icon: 'i-lucide-code',
        content: `
          <h3>REST API v1</h3>
          <p>EngineR предоставляет REST API версии 1 по пути <code>/api/v1</code>.</p>
          <h4>Аутентификация</h4>
          <p>Используется cookie-based сессия. После логина через <code>POST /api/v1/auth/login</code> устанавливается сессионный cookie.</p>
          <h4>Формат ответов</h4>
          <p>Все ответы обёрнуты в envelope:</p>
          <pre><code>{
  "data": { ... },
  "meta": { "request_id": "..." }
}</code></pre>
          <h4>Пагинация</h4>
          <p>Используется cursor-based пагинация. Ответы списков содержат <code>cursor</code> для загрузки следующей страницы.</p>
        `,
      },
      {
        id: 'auth-api',
        title: 'Аутентификация',
        icon: 'i-lucide-code',
        content: `
          <h3>Аутентификация</h3>
          <h4>Логин</h4>
          <pre><code>POST /api/v1/auth/login
{
  "username": "admin",
  "password": "..."
}</code></pre>
          <h4>Контекст пользователя</h4>
          <pre><code>GET /api/v1/auth/me/context</code></pre>
          <p>Возвращает capabilities, tenant, groups текущего пользователя.</p>
          <h4>Выход</h4>
          <pre><code>POST /api/v1/auth/logout</code></pre>
        `,
      },
      {
        id: 'entities-api',
        title: 'Сущности и запросы',
        icon: 'i-lucide-code',
        content: `
          <h3>Сущности и запросы</h3>
          <h4>Создание сущности</h4>
          <pre><code>POST /api/v1/entities
{
  "type_id": "person",
  "fields": {
    "full_name": "Иван Иванов",
    "inn": "1234567890"
  }
}</code></pre>
          <h4>Получение сущности</h4>
          <pre><code>GET /api/v1/entities/{public_id}</code></pre>
          <h4>Поиск сущностей</h4>
          <pre><code>POST /api/v1/entities/query
{
  "from": { "type_id": "person" },
  "search": "Иван",
  "filters": { "inn": "1234567890" }
}</code></pre>
          <h4>Связи</h4>
          <pre><code>GET /api/v1/entities/{public_id}/relations
POST /api/v1/entities/{public_id}/relations
DELETE /api/v1/entities/{public_id}/relations/{relation_id}</code></pre>
        `,
      },
      {
        id: 'checks-risks-api',
        title: 'Проверки и риски',
        icon: 'i-lucide-code',
        content: `
          <h3>Проверки и риски</h3>
          <h4>Проверки</h4>
          <pre><code>GET /api/v1/checks?status=completed&limit=20
POST /api/v1/checks
GET /api/v1/entities/{id}/checks</code></pre>
          <h4>Оценки риска</h4>
          <pre><code>GET /api/v1/assessments?status=approved
GET /api/v1/entities/{id}/assessment
POST /api/v1/entities/{id}/assessment</code></pre>
          <h4>Задачи</h4>
          <pre><code>GET /api/v1/entities/{id}/tasks
POST /api/v1/entities/{id}/tasks</code></pre>
        `,
      },
      {
        id: 'reports-api',
        title: 'Отчёты',
        icon: 'i-lucide-code',
        content: `
          <h3>Отчёты</h3>
          <pre><code>GET /api/v1/reports?status=ready
POST /api/v1/reports
{
  "target_entity_id": "ent-001",
  "template_id": "due_diligence",
  "format": "pdf"
}</code></pre>
        `,
      },
      {
        id: 'webhooks-api',
        title: 'Webhooks',
        icon: 'i-lucide-code',
        content: `
          <h3>Webhooks</h3>
          <p>EngineR может отправлять webhook-уведомления при наступлении событий:</p>
          <ul>
            <li>Создание сущности</li>
            <li>Завершение проверки</li>
            <li>Изменение оценки риска</li>
            <li>Обнаружение изменения при мониторинге</li>
          </ul>
          <p>Webhook отправляется POST-запросом с JSON-телом, содержащим тип события и данные.</p>
        `,
      },
    ],
  },
  {
    title: 'Регламенты',
    icon: 'i-lucide-scale',
    items: [
      {
        id: 'reg-contractor-check',
        title: 'Регламент проверки контрагентов',
        icon: 'i-lucide-scale',
        content: `
          <h3>Регламент проверки контрагентов</h3>
          <ol>
            <li>Создание заявки на проверку контрагента</li>
            <li>Автоматический запуск базовых проверок (санкции, PEP, реестры)</li>
            <li>Оценка риска на основе результатов проверок</li>
            <li>Принятие решения (одобрить / отклонить / эскалировать)</li>
            <li>Уведомление инициатора заявки</li>
            <li>Настройка мониторинга для одобренных контрагентов</li>
          </ol>
        `,
      },
      {
        id: 'reg-sanctions',
        title: 'Регламент работы с санкционными списками',
        icon: 'i-lucide-scale',
        content: `
          <h3>Регламент работы с санкционными списками</h3>
          <ul>
            <li>Проверка по всем актуальным санкционным спискам (UN, EU, US OFAC, РФ)</li>
            <li>При совпадении — автоматическая эскалация на риск-менеджера</li>
            <li>Решение по санкционным совпадениям принимается только ручное</li>
            <li>Мониторинг обновления списков — ежедневно</li>
          </ul>
        `,
      },
      {
        id: 'reg-monitoring',
        title: 'Регламент мониторинга',
        icon: 'i-lucide-scale',
        content: `
          <h3>Регламент мониторинга</h3>
          <p>Мониторинг контрагентов проводится на регулярной основе:</p>
          <ul>
            <li><strong>Высокий риск</strong> — ежемесячно</li>
            <li><strong>Средний риск</strong> — ежеквартально</li>
            <li><strong>Низкий риск</strong> — ежегодно</li>
          </ul>
          <p>При обнаружении изменений (санкции, реестры, негативные СМИ) создаётся уведомление и может быть запущена новая оценка риска.</p>
        `,
      },
      {
        id: 'reg-data-retention',
        title: 'Регламент хранения данных',
        icon: 'i-lucide-scale',
        content: `
          <h3>Регламент хранения данных</h3>
          <p>Сроки хранения данных настраиваются в разделе «Аудит → Хранение»:</p>
          <ul>
            <li>Записи аудита — не менее 5 лет</li>
            <li>Документы контрагентов — весь срок отношений + 3 года</li>
            <li>Результаты проверок — 5 лет</li>
            <li>Оценки риска — 5 лет</li>
          </ul>
          <p>Поддерживается юридический холд (legal hold) — приостановка удаления данных по требованию.</p>
        `,
      },
      {
        id: 'reg-privacy',
        title: 'Политика конфиденциальности',
        icon: 'i-lucide-scale',
        content: `
          <h3>Политика конфиденциальности</h3>
          <p>EngineR обеспечивает защиту персональных данных через:</p>
          <ul>
            <li>Шифрование на уровне БД и при передаче (TLS)</li>
            <li>Маскирование полей в зависимости от прав пользователя</li>
            <li>Аудит всех обращений к данным</li>
            <li>Row-Level Security на уровне БД</li>
            <li>Классификация данных (public, restricted, confidential)</li>
          </ul>
        `,
      },
    ],
  },
];
</script>
