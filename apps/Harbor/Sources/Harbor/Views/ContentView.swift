import SwiftUI

struct ContentView: View {
  @Bindable var store: HarborStore
  @Environment(\.scenePhase) private var phase

  var body: some View {
    NavigationSplitView {
      List(
        selection: Binding(
          get: { store.selection },
          set: { value in
            store.selection = value
            store.report = nil
            store.message = ""
            store.error = nil
          })
      ) {
        Section(store.text("实例")) {
          ForEach(store.profiles) { item in
            Label {
              VStack(alignment: .leading, spacing: 3) {
                Text(item.id).fontWeight(.medium)
                Text(item.issue == nil ? store.text(key: item.status.title) : store.text("需要检查"))
                  .font(.caption).foregroundStyle(.secondary)
              }
            } icon: {
              ProfileIconView(appPath: item.profile.app_bundle, revision: store.iconRevision).frame(
                width: 22, height: 22)
            }
            .tag(item.id)
          }
        }
      }
      .listStyle(.sidebar)
      .disabled(store.busy)
      .navigationSplitViewColumnWidth(min: 190, ideal: 230)
      .safeAreaInset(edge: .bottom) {
        Button {
          store.creating = true
        } label: {
          Label(store.text("创建副本"), systemImage: "plus")
            .frame(maxWidth: .infinity, alignment: .leading)
        }
        .buttonStyle(.borderless).padding().disabled(store.busy)
      }
    } detail: {
      VStack(spacing: 0) {
        if store.registry.hasPrefix("/private/tmp/") || store.registry.hasPrefix("/tmp/") {
          Label(store.text("当前使用临时数据目录，正式使用前请将实例迁移到持久目录。"), systemImage: "exclamationmark.triangle")
            .font(.callout).frame(maxWidth: .infinity, alignment: .leading)
            .padding(12).background(.orange.opacity(0.1))
        }
        if let selected = store.selected {
          DetailView(store: store, item: selected)
        } else {
          ContentUnavailableView {
            Label(store.text("从一个新副本开始"), systemImage: "macwindow.badge.plus")
          } description: {
            Text(store.text("选择官方原版，Harbor 会创建独立副本和空的数据目录。"))
          } actions: {
            Button(store.text("创建第一个副本")) { store.creating = true }
              .buttonStyle(.borderedProminent).disabled(store.busy)
          }
        }
        if let notice = store.removalNotice {
          VStack(alignment: .leading, spacing: 6) {
            Label(store.text(notice), systemImage: "checkmark.circle")
            if let retained = store.retainedData {
              Text(retained).font(.caption).textSelection(.enabled)
              Button(store.text("查看保留的数据")) { store.reveal(retained) }.buttonStyle(.link)
            }
          }.frame(maxWidth: .infinity, alignment: .leading).padding()
        }
        if let error = store.error, !store.creating {
          VStack(alignment: .leading, spacing: 6) {
            Label(store.text("操作未完成"), systemImage: "exclamationmark.triangle").font(.headline)
            Text(store.text(error)).textSelection(.enabled)
            Text(store.text("请检查上述原因，调整后重试；也可以使用“检查”查看详情。"))
              .font(.caption).foregroundStyle(.secondary)
          }
          .frame(maxWidth: .infinity, alignment: .leading)
          .padding().background(.orange.opacity(0.1))
        }
        Divider()
        HStack(spacing: 10) {
          if store.busy {
            ProgressView().controlSize(.small)
            Text(store.text(store.activity))
          } else {
            Text(store.text("\(store.profiles.count) 个实例")).foregroundStyle(.secondary)
          }
          Spacer()
          Text(store.registry).font(.caption).foregroundStyle(.secondary)
            .lineLimit(1).truncationMode(.middle).help(store.registry)
        }.padding(12)
      }
    }
    .toolbar {
      ToolbarItem(placement: .primaryAction) {
        LanguagePicker(language: store.language).fixedSize()
      }
      ToolbarItemGroup {
        Button {
          Task { await store.refresh() }
        } label: {
          Label(store.text("刷新"), systemImage: "arrow.clockwise")
        }
        Button {
          store.creating = true
        } label: {
          Label(store.text("创建副本"), systemImage: "plus")
        }
      }
    }
    .disabled(store.busy && !store.creating)
    .sheet(isPresented: $store.creating) { CreateProfileView(store: store) }
    .task { await store.refresh() }
    .onChange(of: phase) { _, value in
      if value == .active { Task { await store.refresh() } }
    }
  }
}
