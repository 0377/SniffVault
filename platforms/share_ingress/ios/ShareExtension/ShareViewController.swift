import UIKit
import MobileCoreServices

class ShareViewController: UIViewController {
    private let urlTypeIdentifier = kUTTypeURL as String
    private let plainTextTypeIdentifier = kUTTypePlainText as String

    private var didProcess = false

    override func viewDidAppear(_ animated: Bool) {
        super.viewDidAppear(animated)
        guard !didProcess else { return }
        didProcess = true
        processInput()
    }

    private func processInput() {
        guard let extensionItem = extensionContext?.inputItems.first as? NSExtensionItem,
              let attachments = extensionItem.attachments
        else {
            showFailure()
            return
        }

        for attachment in attachments {
            if attachment.hasItemConformingToTypeIdentifier(urlTypeIdentifier) {
                loadText(from: attachment, typeIdentifier: urlTypeIdentifier) { [weak self] value in
                    self?.openMainApp(with: value)
                }
                return
            }
            if attachment.hasItemConformingToTypeIdentifier(plainTextTypeIdentifier) {
                loadText(from: attachment, typeIdentifier: plainTextTypeIdentifier) { [weak self] value in
                    self?.openMainApp(with: value)
                }
                return
            }
        }

        showFailure()
    }

    private func loadText(
        from attachment: NSItemProvider,
        typeIdentifier: String,
        completion: @escaping (String) -> Void
    ) {
        attachment.loadItem(forTypeIdentifier: typeIdentifier, options: nil) { [weak self] item, _ in
            DispatchQueue.main.async {
                switch item {
                case let url as URL:
                    completion(url.absoluteString)
                case let text as String:
                    completion(text)
                default:
                    self?.showFailure()
                }
            }
        }
    }

    private func openMainApp(with raw: String) {
        let trimmed = raw.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else {
            showFailure()
            return
        }
        var components = URLComponents()
        components.scheme = "sniffvault"
        components.host = "add"
        components.queryItems = [URLQueryItem(name: "url", value: trimmed)]
        guard let url = components.url else {
            showFailure()
            return
        }
        extensionContext?.open(url) { _ in
            self.extensionContext?.completeRequest(returningItems: nil)
        }
    }

    private func showFailure() {
        view.backgroundColor = .systemBackground

        let label = UILabel()
        label.text = "未识别到有效链接"
        label.textAlignment = .center
        label.numberOfLines = 0
        label.translatesAutoresizingMaskIntoConstraints = false
        view.addSubview(label)

        NSLayoutConstraint.activate([
            label.centerXAnchor.constraint(equalTo: view.centerXAnchor),
            label.centerYAnchor.constraint(equalTo: view.centerYAnchor),
            label.leadingAnchor.constraint(greaterThanOrEqualTo: view.leadingAnchor, constant: 16),
            label.trailingAnchor.constraint(lessThanOrEqualTo: view.trailingAnchor, constant: -16),
        ])

        DispatchQueue.main.asyncAfter(deadline: .now() + 1.5) { [weak self] in
            self?.extensionContext?.completeRequest(returningItems: nil)
        }
    }
}
